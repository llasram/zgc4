use std::io::{self, Write};

use board::{Board, LegalMove, Move, Side};
use player::Player;

pub struct HumanPlayer;

impl HumanPlayer {
    fn get_side(_: &Board) -> Side {
        loop {
            print!("Side {{n,e,s,w}}? ");
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

    fn get_pos(b: &Board) -> usize {
        loop {
            print!("Position [0-{}]? ", b.size() - 1);
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
            let side = HumanPlayer::get_side(b);
            let pos = HumanPlayer::get_pos(b);
            match Move::new(side, pos).annotated(b) {
                Some(m) => return m,
                None => println!("Illegal move!"),
            }
        }
    }
}
