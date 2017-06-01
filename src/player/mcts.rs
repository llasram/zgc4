use std::iter;

use rand::{self, Rng};

use board::{Board, LegalMove, GameState};
use player::Player;

const PRIOR_SMOOTH: usize = 500;

pub struct MCTSPlayer;

impl Player for MCTSPlayer {
    fn choose(&self, b: &Board) -> LegalMove {
        let mut rng = rand::thread_rng();
        let mut node = Node::Unvisited;
        for _ in 0..100000 { node.explore(&mut rng, b.clone()); }
        node.best_move(b)
    }
}

#[derive(Clone, Debug)]
enum Node {
    Unvisited,
    CertainLoss,
    CertainWin(usize),
    CertainDraw(usize),
    Probabilistic(Probabilistic),
}

impl Node {
    pub fn best_move(&self, b: &Board) -> LegalMove {
        match *self {
            Node::Unvisited => panic!("node is unvisited"),
            Node::CertainLoss => b.legal_moves_iter().next().unwrap(),
            Node::CertainWin(i) => b.legal_moves_iter().nth(i).unwrap(),
            Node::CertainDraw(i) => b.legal_moves_iter().nth(i).unwrap(),
            Node::Probabilistic(ref p) => p.best_move(b),
        }
    }

    pub fn explore<R: Rng>(&mut self, rng: &mut R, b: Board) -> isize {
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

    fn score(&self) -> isize {
        match *self {
            Node::Unvisited => panic!("node is unvisited"),
            Node::CertainLoss => -1,
            Node::CertainWin(..) => 1,
            Node::CertainDraw(..) => 0,
            Node::Probabilistic(ref p) => p.score(),
        }
    }

    fn explore_unvisted<R: Rng>(&mut self, rng: &mut R, mut b: Board) -> Result<isize, Node> {
        let (n, i, m) = Node::choose_unvisited_first(rng, &b);
        match b.make_legal_move(m) {
            GameState::Won => Err(Node::CertainWin(i)),
            GameState::Drawn => Err(Node::CertainDraw(i)),
            GameState::Ongoing => {
                let score = Node::choose_unvisited_rest(rng, b);
                Err(Node::Probabilistic(Probabilistic::new(n, score)))
            }
        }
    }

    fn choose_unvisited_first<R: Rng>(rng: &mut R, b: &Board) -> (usize, usize, LegalMove) {
        let mut iter = b.legal_moves_iter();
        let mut m = iter.next().unwrap();
        if m.is_winning() { return (0, 0, m); }
        let mut i = 0;
        let mut n = 1;
        for (i1, m1) in iter.enumerate() {
            n += 1;
            if m1.is_winning() { return (0, i1, m1); }
            if rng.gen_range(0, n) == 0 {
                i = i1;
                m = m1;
            }
        }
        (n, i, m)
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
            Node::CertainLoss => 1.0,
            Node::CertainWin(..) => 0.0,
            Node::CertainDraw(..) => 0.0,
            Node::Probabilistic(ref p) => p.p_parent_win(),
        }
    }
}

#[derive(Clone, Debug)]
struct Probabilistic  {
    nwin: usize,
    nplay: usize,
    children: Box<[Node]>,
}

impl Probabilistic {
    fn new(nchildren: usize, score: isize) -> Self {
        let nwin = PRIOR_SMOOTH + if score > 0 { 1 } else { 0 };
        let nplay = PRIOR_SMOOTH + PRIOR_SMOOTH + 1;
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

    fn explore<R: Rng>(&mut self, rng: &mut R, mut b: Board) -> Result<isize, Node> {
        let mut nwin = 0;
        let mut ndraw = 0;
        let mut idraw = 0;
        let mut total = 0.0f64;
        for (i, node) in self.children.iter().enumerate() {
            match *node {
                Node::CertainLoss => return Err(Node::CertainWin(i)),
                Node::CertainWin(..) => nwin += 1,
                Node::CertainDraw(..) => { ndraw += 1; idraw = i; },
                _ => total += node.p_parent_win(),
            }
        }
        let n = self.children.len();
        if nwin == n  {
            Err(Node::CertainLoss)
        } else if ndraw + nwin == n {
            Err(Node::CertainDraw(idraw))
        } else {
            let target = rng.next_f64();
            let (_, node, m) = self.children.iter_mut().zip(b.legal_moves_iter()).
                scan(0.0, |p, (node, m)| {
                    *p += node.p_parent_win() / total;
                    Some((*p, node, m))
                }).find(|&(p, _, _)| {
                    p > target
                }).unwrap();
            b.make_legal_move(m);
            let score = -node.explore(rng, b);
            self.nwin += if score > 0 { 1 } else { 0 };
            self.nplay += 1;
            Ok(score)
        }
    }

    fn score(&self) -> isize {
        let half = self.nplay / 2;
        if self.nwin > half { 1 }
        else if self.nwin == half { 0 }
        else { -1 }
    }
}
