mod random;

use rand::Rng;

use board::{Board, LegalMove};

pub trait Player {
    fn choose(&self, b: &Board) -> LegalMove;
}

fn choose_winning_or_random<R: Rng>(b: &Board, rng: &mut R) -> LegalMove {
    let mut iter = b.legal_moves_iter();
    let mut m = iter.next().unwrap();
    if m.is_winning() { return m; }
    for (i, m1) in iter.enumerate() {
        if m1.is_winning() { return m1; }
        if rng.gen_range(0, i + 2) == 0 { m = m1; }
    }
    m
}

pub use self::random::RandomPlayer;
