use rand;

use board::{Board, LegalMove};
use player::Player;

pub struct RandomPlayer;

impl Player for RandomPlayer {
    fn choose(&self, b: &Board) -> LegalMove {
        super::choose_winning_or_random(b, &mut rand::thread_rng())
    }
}
