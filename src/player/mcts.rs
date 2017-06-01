use std::cmp;
use std::iter;
use std::time::{Duration, Instant};

use rand::{self, Rng};
use rand::distributions::IndependentSample;
use rand::distributions::gamma::Gamma;

use board::{Board, LegalMove, GameState};
use player::Player;

// Jeffrey's prior
const PRIOR: f64 = 0.5;

pub struct MCTSPlayer {
    dur: Duration,
}

impl MCTSPlayer {
    pub fn new(dur: Duration) -> Self {
        MCTSPlayer { dur }
    }
}

impl Player for MCTSPlayer {
    fn choose(&self, b: &Board) -> LegalMove {
        let now = Instant::now();
        let mut rng = rand::thread_rng();
        let mut node = Node::Unvisited;
        for i in 0.. {
            node.explore(&mut rng, b.clone());
            if node.is_certain() { break; }
            if now.elapsed() >= self.dur {
                println!("Choosing move after {} play-throughs", i);
                break;
            }
        }
        node.best_move(b)
    }
}

#[derive(Clone, Debug)]
enum Node {
    Unvisited,
    CertainLoss(Certain),
    CertainWin(Certain),
    CertainDraw(Certain),
    Probabilistic(Probabilistic),
}

impl Node {
    pub fn is_certain(&self) -> bool {
        match *self {
            Node::Unvisited => false,
            Node::CertainLoss(..) => true,
            Node::CertainWin(..) => true,
            Node::CertainDraw(..) => true,
            Node::Probabilistic(..) => false,
        }
    }

    pub fn best_move(&self, b: &Board) -> LegalMove {
        match *self {
            Node::Unvisited => panic!("node is unvisited"),
            Node::CertainLoss(ref c) => {
                println!("Certain loss in {} move(s)", c.depth);
                c.best_move(b)
            },
            Node::CertainWin(ref c) => {
                println!("Certain win in {} move(s)", c.depth);
                c.best_move(b)
            },
            Node::CertainDraw(ref c) => c.best_move(b),
            Node::Probabilistic(ref p) => p.best_move(b),
        }
    }

    pub fn explore<R: Rng>(&mut self, rng: &mut R, b: Board) -> f64 {
        let result = match *self {
            Node::Unvisited => self.explore_unvisted(rng, b),
            Node::Probabilistic(ref mut p) => p.explore(rng, b),
            _ => Ok(self.score()),
        };
        match result {
            Ok(score) => score,
            Err(node) => {
                *self = node;
                self.score()
            },
        }
    }

    fn score(&self) -> f64 {
        match *self {
            Node::Unvisited => panic!("node is unvisited"),
            Node::CertainLoss(..) => 1.0,
            Node::CertainWin(..) => 0.0,
            Node::CertainDraw(..) => 0.5,
            Node::Probabilistic(ref p) => p.score(),
        }
    }

    fn explore_unvisted<R: Rng>(&mut self, rng: &mut R, mut b: Board)
                                -> Result<f64, Node> {
        let (n, i, m) = Node::choose_unvisited_first(rng, &b);
        match b.make_legal_move(m) {
            GameState::Won => Err(Node::CertainWin(Certain::new(1, i))),
            GameState::Drawn => Err(Node::CertainDraw(Certain::new(1, i))),
            GameState::Ongoing => {
                let score = Node::choose_unvisited_rest(rng, b);
                Err(Node::Probabilistic(Probabilistic::new(n, score)))
            }
        }
    }

    fn choose_unvisited_first<R: Rng>(rng: &mut R, b: &Board) -> (usize, usize, LegalMove) {
        let mut m = None;
        let mut i = 0;
        let mut n = 0;
        for (i1, m1) in b.legal_moves_iter().enumerate() {
            n += 1;
            if m1.is_winning() { return (n, i1, m1); }
            if rng.gen_range(0, n) == 0 {
                i = i1;
                m = Some(m1);
            }
        }
        (n, i, m.unwrap())
    }

    fn choose_unvisited_rest<R: Rng>(rng: &mut R, mut b: Board) -> f64 {
        let mut score = 1.0;
        loop {
            let m = super::choose_winning_or_random(&b, rng);
            match b.make_legal_move(m) {
                GameState::Won => return score,
                GameState::Drawn => return 0.5,
                GameState::Ongoing => score = 1.0 - score,
            }
        }
    }

    fn p_parent_win(&self) -> f64 {
        match *self {
            Node::Unvisited => 0.5,
            Node::CertainLoss(..) => 1.0,
            Node::CertainWin(..) => 0.0,
            Node::CertainDraw(..) => 0.5,
            Node::Probabilistic(ref p) => p.p_parent_win(),
        }
    }

    fn p_parent_win_sample<R: Rng>(&self, r: &mut R) -> f64 {
        match *self {
            Node::Probabilistic(ref p) => p.p_parent_win_sample(r),
            _ => self.p_parent_win(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Certain {
    depth: usize,
    index: usize,
}

impl Certain {
    fn new(depth: usize, index: usize) -> Self {
        Certain { depth, index }
    }

    fn parent(&self, index: usize) -> Self {
        Certain::new(self.depth + 1, index)
    }

    fn best_move(&self, b: &Board) -> LegalMove {
        b.legal_moves_iter().nth(self.index).unwrap()
    }
}


#[derive(Clone, Debug)]
struct Probabilistic {
    score: f64,
    nplay: f64,
    children: Box<[Node]>,
}

impl Probabilistic {
    fn new(nchildren: usize, score: f64) -> Self {
        let score = PRIOR + score;
        let nplay = PRIOR + PRIOR + 1.0;
        let children = iter::repeat(Node::Unvisited).
            take(nchildren).collect::<Vec<Node>>().into_boxed_slice();
        Probabilistic { score, nplay, children }
    }

    fn best_move(&self, b: &Board) -> LegalMove {
        self.children.iter().zip(b.legal_moves_iter()).max_by(|&(n1, _), &(n2, _)| {
            n1.p_parent_win().partial_cmp(&n2.p_parent_win()).unwrap()
        }).map(|(_, m)| m).unwrap()
    }

    fn p_parent_win(&self) -> f64 {
        self.score / self.nplay as f64
    }

    fn p_parent_win_sample<R: Rng>(&self, r: &mut R) -> f64 {
        // Sample from Beta distribution via Gamma samples
        let a = Gamma::new(self.score, 1.0).ind_sample(r);
        let b = Gamma::new(self.nplay as f64, 1.0).ind_sample(r);
        a / (a + b)
    }

    fn explore<R: Rng>(&mut self, rng: &mut R, mut b: Board)
                       -> Result<f64, Node> {
        let mut closs = None;
        let mut nwin = 0;
        let mut cwin = None;
        let mut ndraw = 0;
        let mut cdraw = None;
        for (i, node) in self.children.iter().enumerate() {
            match *node {
                Node::CertainLoss(ref c1) => {
                    match closs {
                        None => closs = Some(c1.parent(i)),
                        Some(ref mut c) => *c = cmp::min(*c, c1.parent(i)),
                    }
                },
                Node::CertainWin(ref c1) => {
                    nwin += 1;
                    match cwin {
                        None => cwin = Some(c1.parent(i)),
                        Some(ref mut c) => *c = cmp::max(*c, c1.parent(i)),
                    }
                },
                Node::CertainDraw(ref c1) => {
                    ndraw += 1;
                    match cdraw {
                        None => cdraw = Some(c1.parent(i)),
                        Some(ref mut c) => *c = cmp::max(*c, c1.parent(i)),
                    }
                },
                _ => (),
            }
        }
        let n = self.children.len();
        if let Some(c) = closs { return Err(Node::CertainWin(c)); }
        if let Some(c) = cwin { if n == nwin { return Err(Node::CertainLoss(c)); } }
        if let Some(c) = cdraw { if n == nwin + ndraw { return Err(Node::CertainDraw(c)); } }
        let (_, node, m) = self.children.iter_mut().zip(b.legal_moves_iter()).
            map(|(node, m)| (node.p_parent_win_sample(rng), node, m)).
            max_by(|&(p1, _, _), &(p2, _, _)| p1.partial_cmp(&p2).unwrap()).
            unwrap();
        b.make_legal_move(m);
        let score = 1.0 - node.explore(rng, b);
        self.score += score;
        self.nplay += 1.0;
        Ok(score)
    }

    fn score(&self) -> f64 {
        self.score - PRIOR as f64
    }
}
