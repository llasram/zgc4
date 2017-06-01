use std::cmp;
use std::iter;

use rand::{self, Rng};

use board::{Board, LegalMove, GameState};
use player::Player;

pub struct MCTSPlayer {
    niter: usize,
    prior: usize,
}

impl MCTSPlayer {
    pub fn new(niter: usize, prior: usize) -> Self {
        MCTSPlayer { niter, prior }
    }
}

impl Player for MCTSPlayer {
    fn choose(&self, b: &Board) -> LegalMove {
        let mut rng = rand::thread_rng();
        let mut node = Node::Unvisited;
        for _ in 0..self.niter {
            node.explore(&mut rng, b.clone(), self.prior);
            if node.is_certain() { break; }
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

    pub fn explore<R: Rng>(&mut self, rng: &mut R, b: Board, prior: usize) -> isize {
        let result = match *self {
            Node::Unvisited => self.explore_unvisted(rng, b, prior),
            Node::Probabilistic(ref mut p) => p.explore(rng, b, prior),
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

    fn score(&self) -> isize {
        match *self {
            Node::Unvisited => panic!("node is unvisited"),
            Node::CertainLoss(..) => -1,
            Node::CertainWin(..) => 1,
            Node::CertainDraw(..) => 0,
            Node::Probabilistic(ref p) => p.score(),
        }
    }

    fn explore_unvisted<R: Rng>(&mut self, rng: &mut R, mut b: Board, prior: usize)
                                -> Result<isize, Node> {
        let (n, i, m) = Node::choose_unvisited_first(rng, &b);
        match b.make_legal_move(m) {
            GameState::Won => Err(Node::CertainWin(Certain::new(1, i))),
            GameState::Drawn => Err(Node::CertainDraw(Certain::new(1, i))),
            GameState::Ongoing => {
                let score = Node::choose_unvisited_rest(rng, b);
                Err(Node::Probabilistic(Probabilistic::new(n, prior, score)))
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

    fn choose_unvisited_rest<R: Rng>(rng: &mut R, mut b: Board) -> isize {
        let mut score = -1;
        loop {
            let m = super::choose_winning_or_random(&b, rng);
            match b.make_legal_move(m) {
                GameState::Won => return score,
                GameState::Drawn => return 0,
                GameState::Ongoing => score = -score,
            }
        }
    }

    fn p_parent_win(&self) -> f64 {
        match *self {
            Node::Unvisited => 0.5,
            Node::CertainLoss(..) => 1.0,
            Node::CertainWin(..) => 0.0,
            Node::CertainDraw(..) => 0.0,
            Node::Probabilistic(ref p) => p.p_parent_win(),
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
    nwin: usize,
    nplay: usize,
    children: Box<[Node]>,
}

impl Probabilistic {
    fn new(nchildren: usize, prior: usize, score: isize) -> Self {
        let nwin = prior + if score > 0 { 1 } else { 0 };
        let nplay = prior + prior + 1;
        let children = iter::repeat(Node::Unvisited).
            take(nchildren).collect::<Vec<Node>>().into_boxed_slice();
        Probabilistic { nwin, nplay, children }
    }

    fn best_move(&self, b: &Board) -> LegalMove {
        self.children.iter().zip(b.legal_moves_iter()).max_by(|&(n1, _), &(n2, _)| {
            n1.p_parent_win().partial_cmp(&n2.p_parent_win()).unwrap()
        }).map(|(_, m)| m).unwrap()
    }

    fn p_parent_win(&self) -> f64 {
        (self.nplay - self.nwin) as f64 / self.nplay as f64
    }

    fn explore<R: Rng>(&mut self, rng: &mut R, mut b: Board, prior: usize)
                       -> Result<isize, Node> {
        let mut closs = None;
        let mut nwin = 0;
        let mut cwin = None;
        let mut ndraw = 0;
        let mut cdraw = None;
        let mut total = 0.0f64;
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
                _ => total += node.p_parent_win(),
            }
        }
        let n = self.children.len();
        if let Some(c) = closs { return Err(Node::CertainWin(c)); }
        if let Some(c) = cwin { if n == nwin { return Err(Node::CertainLoss(c)); } }
        if let Some(c) = cdraw { if n == nwin + ndraw { return Err(Node::CertainDraw(c)); } }
        let target = rng.next_f64();
        let (_, node, m) = self.children.iter_mut().zip(b.legal_moves_iter()).
            scan(0.0, |p, (node, m)| {
                *p += node.p_parent_win() / total;
                Some((*p, node, m))
            }).find(|&(p, _, _)| {
                p > target
            }).unwrap();
        b.make_legal_move(m);
        let score = -node.explore(rng, b, prior);
        self.nwin += if score > 0 { 1 } else { 0 };
        self.nplay += 1;
        Ok(score)
    }

    fn score(&self) -> isize {
        let half = self.nplay / 2;
        if self.nwin > half { 1 }
        else if self.nwin == half { 0 }
        else { -1 }
    }
}
