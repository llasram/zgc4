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
enum Finding {
    Score(f64),
    Replace(Node),
    Both(Node, f64),
}

#[derive(Clone, Debug, PartialEq)]
enum Node {
    Unvisited,
    Probabilistic(Probabilistic),
    CertainLoss(Certain),
    CertainWin(Certain),
    CertainDraw(Certain),
}

impl Node {
    pub fn is_certain(&self) -> bool {
        match *self {
            Node::Unvisited => false,
            Node::Probabilistic(..) => false,
            Node::CertainLoss(..) => true,
            Node::CertainWin(..) => true,
            Node::CertainDraw(..) => true,
        }
    }

    pub fn best_move(&self, b: &Board) -> LegalMove {
        match *self {
            Node::Unvisited => panic!("node is unvisited"),
            Node::Probabilistic(ref p) => p.best_move(b),
            Node::CertainLoss(ref c) => {
                println!("Certain loss in {} move(s)", c.depth);
                c.best_move(b)
            },
            Node::CertainWin(ref c) => {
                println!("Certain win in {} move(s)", c.depth);
                c.best_move(b)
            },
            Node::CertainDraw(ref c) => c.best_move(b),
        }
    }

    pub fn explore<R: Rng>(&mut self, rng: &mut R, b: Board) -> f64 {
        let result = match *self {
            Node::Unvisited => self.explore_unvisted(rng, b),
            Node::Probabilistic(ref mut p) => p.explore(rng, b),
            _ => Finding::Score(self.score()),
        };
        match result {
            Finding::Score(score) => score,
            Finding::Replace(node) => { *self = node; self.score() },
            Finding::Both(node, score) => { *self = node; score }
        }
    }

    fn score(&self) -> f64 {
        match *self {
            Node::Unvisited => panic!("node is unvisited"),
            Node::Probabilistic(..) => panic!("node is probabilistic"),
            Node::CertainLoss(..) => 1.0,
            Node::CertainWin(..) => 0.0,
            Node::CertainDraw(..) => 0.5,
        }
    }

    fn explore_unvisted<R: Rng>(&mut self, rng: &mut R, mut b: Board) -> Finding {
        let (n, i, m) = Node::choose_unvisited_first(rng, &b);
        match b.make_legal_move(m) {
            GameState::Won => Finding::Replace(Node::CertainWin(Certain::new(1, i))),
            GameState::Drawn => Finding::Replace(Node::CertainDraw(Certain::new(1, i))),
            GameState::Ongoing => {
                let score = Node::choose_unvisited_rest(rng, b);
                let node = Node::Probabilistic(Probabilistic::new(n, score));
                Finding::Both(node, score)
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

    fn expected_score(&self) -> f64 {
        match *self {
            Node::Unvisited => 0.5,
            Node::Probabilistic(ref p) => p.expected_score(),
            Node::CertainLoss(..) => 1.0,
            Node::CertainWin(..) => 0.0,
            Node::CertainDraw(..) => 0.5,
        }
    }

    fn expected_score_sample<R: Rng>(&self, rng: &mut R) -> f64 {
        match *self {
            Node::Unvisited => beta_sample(rng, PRIOR, PRIOR),
            Node::Probabilistic(ref p) => p.expected_score_sample(rng),
            _ => self.expected_score(),
        }
    }

    fn rank_ordinal(&self) -> usize {
        match *self {
            Node::Unvisited => 2,
            Node::Probabilistic(..) => 2,
            Node::CertainLoss(..) => 3,
            Node::CertainWin(..) => 0,
            Node::CertainDraw(..) => 1,
        }
    }

    fn rank_discriminator(&self) -> isize {
        match *self {
            Node::Unvisited => 0,
            Node::Probabilistic(..) => 0,
            Node::CertainLoss(ref c) => -(c.depth as isize),
            Node::CertainWin(ref c) => c.depth as isize,
            Node::CertainDraw(ref c) => c.depth as isize,
        }
    }

    fn rank_key<R: Rng>(&self, rng: &mut R) -> (usize, f64, isize) {
        let o = self.rank_ordinal();
        let p = self.expected_score_sample(rng);
        let d = self.rank_discriminator();
        (o, p, d)
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


#[derive(Clone, Debug, PartialEq)]
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
            n1.expected_score().partial_cmp(&n2.expected_score()).unwrap()
        }).map(|(_, m)| m).unwrap()
    }

    fn expected_score(&self) -> f64 {
        self.score / self.nplay
    }

    fn expected_score_sample<R: Rng>(&self, rng: &mut R) -> f64 {
        beta_sample(rng, self.score, self.nplay - self.score)
    }

    fn explore<R: Rng>(&mut self, rng: &mut R, mut b: Board) -> Finding {
        let (_, i, node) = self.children.iter_mut().enumerate().map(|(i, node)| {
            (node.rank_key(rng), i, node)
        }).max_by(|&(k1, _, _), &(k2, _, _)| {
            k1.partial_cmp(&k2).unwrap()
        }).unwrap();
        match *node {
            Node::CertainLoss(ref c) => Finding::Replace(Node::CertainWin(c.parent(i))),
            Node::CertainWin(ref c) => Finding::Replace(Node::CertainLoss(c.parent(i))),
            Node::CertainDraw(ref c) => Finding::Replace(Node::CertainDraw(c.parent(i))),
            _ => {
                let m = b.legal_moves_iter().nth(i).unwrap();
                b.make_legal_move(m);
                let score = 1.0 - node.explore(rng, b);
                self.score += score;
                self.nplay += 1.0;
                Finding::Score(score)
            }
        }
    }
}

fn beta_sample<R: Rng>(rng: &mut R, alpha: f64, beta: f64) -> f64 {
    if alpha <= 1.0 && beta <= 1.0 {
        loop {
            let u = rng.next_f64();
            let v = rng.next_f64();
            let x = u.powf(1.0 / alpha);
            let y = v.powf(1.0 / beta);

            if x + y > 1.0 { continue; }
            if x + y > 0.0 { return x / (x + y); }

            let ln_x = u.ln() / alpha;
            let ln_y = v.ln() / beta;
            let ln_m = if ln_x > ln_y { ln_x } else { ln_y };
            let ln_x = ln_x - ln_m;
            let ln_y = ln_y - ln_m;
            return (ln_x - (ln_x.exp() + ln_y.exp()).ln()).exp();
        }
    } else {
        let a = Gamma::new(alpha, 1.0).ind_sample(rng);
        let b = Gamma::new(beta, 1.0).ind_sample(rng);
        a / (a + b)
    }
}
