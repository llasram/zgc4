use std::io::{self, Write};

use board::{Board, LegalMove, Move, Side};
use player::Player;

pub struct HumanPlayer;

impl HumanPlayer {
    fn get_side() -> Side {
        loop {
            print!("Side? ");
            io::stdout().flush().unwrap();
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer).unwrap();
            match buffer.trim() {
                "n" => return Side::North,
                "e" => return Side::East,
                "s" => return Side::South,
                "w" => return Side::West,
                _ => println!("Invalid side!"),
            }
        }
    }

    fn get_pos() -> usize {
        loop {
            print!("Position? ");
            io::stdout().flush().unwrap();
            let mut buffer = String::new();
            io::stdin().read_line(&mut buffer).unwrap();
            match buffer.trim().parse::<usize>() {
                Ok(pos) => return pos,
                Err(..) => println!("Invalid position!"),
            }
        }
    }
}

impl Player for HumanPlayer {
    fn choose(&self, b: &Board) -> LegalMove {
        loop {
            let side = HumanPlayer::get_side();
            let pos = HumanPlayer::get_pos();
            match Move::new(side, pos).annotated(b) {
                Some(m) => return m,
                None => println!("Illegal move!"),
            }
        }
    }
}
