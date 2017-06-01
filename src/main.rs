extern crate itertools;
extern crate num_traits;
extern crate smallvec;
extern crate rand;

pub mod board;
pub mod player;

use board::{Board, GameState};
use player::Player;

fn main() {
    let mut b = Board::generate(10, 6);
    let players: [Box<Player>; 2] = [Box::new(player::MCTSPlayer::new(100000, 100)),
                                     Box::new(player::MCTSPlayer::new(1000000, 100))];
    println!("{}", b);
    for p in players.iter().cycle() {
        let m = p.choose(&b);
        let r = b.make_legal_move(m);
        println!("{}", b);
        match r {
            GameState::Ongoing => (),
            GameState::Drawn => {
                println!("Drawn.");
                break;
            },
            GameState::Won => {
                println!("Won! ({})", b.active());
                break;
            },
        }
    }
}
