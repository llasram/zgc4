extern crate itertools;
extern crate num_traits;
extern crate smallvec;
extern crate rand;

pub mod board;
pub mod player;

use std::time::Duration;

use board::{Board, GameState};
use player::Player;

fn main() {
    let mut b = Board::generate(10, 6);
    let dur = Duration::new(5, 0);
    let players: [Box<Player>; 2] = [
        Box::new(player::HumanPlayer),
        //Box::new(player::MCTSPlayer::new(dur)),
        Box::new(player::MCTSPlayer::new(dur)),
    ];
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
