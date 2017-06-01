#[macro_use] extern crate enum_primitive;
extern crate itertools;
extern crate num_traits;
extern crate smallvec;
extern crate rand;

mod board;
mod player;

use std::cmp;
use std::iter;
use std::io::{self, Write};
use std::ops::Range;
use std::slice;

use rand::Rng;
use num_traits::FromPrimitive;
use itertools::Itertools;

const ENTRY_BITS: usize = 2;
const ENTRY_MASK: u32 = !(!0u32 << ENTRY_BITS);
const BOARD_WORD_BITS: usize = 32;
const BOARD_WORD_ENTRIES: usize = BOARD_WORD_BITS / ENTRY_BITS;
const BOARD_WORDS: usize = 16;
const PRIOR_SMOOTH: u64 = 500;

enum_from_primitive! {
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    Empty = 0,
    Player1 = 1,
    Player2 = 2,
    Block = 3
}
}

impl Entry {
    pub fn pretty(&self) -> &'static str {
        match *self {
            Entry::Empty => ".",
            Entry::Player1 => "1",
            Entry::Player2 => "2",
            Entry::Block => "X",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    North,
    East,
    South,
    West,
}

pub type SideMovesIter = iter::Map<iter::Zip<iter::Repeat<Side>, Range<usize>>,
                                   fn((Side, usize)) -> Move>;

impl Side {
    pub fn side_moves((&side, dim): (&Side, usize)) -> SideMovesIter {
        iter::repeat(side).zip(0..dim).map(Move::from_tuple)
    }
}

static SIDES: [Side; 4] = [Side::North, Side::East, Side::South, Side::West];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move {
    pub side: Side,
    pos: u32,
}

impl Move {
    pub fn new(side: Side, pos: usize) -> Move {
        Move { side: side, pos: pos as u32 }
    }

    pub fn from_tuple((side, pos): (Side, usize)) -> Move {
        Move { side: side, pos: pos as u32 }
    }

    pub fn pos(&self) -> usize {
        self.pos as usize
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Board {
    size: u32,
    data: [u32; BOARD_WORDS],
}

impl Board {
    pub fn new(size: usize) -> Board {
        Board { size: size as u32, data: [0; BOARD_WORDS] }
    }

    pub fn dim(&self) -> usize {
        self.size as usize
    }

    pub fn len(&self) -> usize {
        self.dim() * self.dim()
    }

    pub fn set(self, x: usize, y: usize, v: Entry) -> Board {
        assert!(x < self.dim());
        assert!(y < self.dim());
        unsafe { self.set_unchecked(x, y, v as u32) }
    }

    pub unsafe fn set_unchecked(mut self, x: usize, y: usize, v: u32) -> Board {
        let i = x + y * (self.dim());
        let j = i / BOARD_WORD_ENTRIES;
        let k = ENTRY_BITS * (i % BOARD_WORD_ENTRIES);
        let v = (v as u32) << k;
        let m = !(ENTRY_MASK << k);
        {
            let e = self.data.get_unchecked_mut(j);
            *e = (*e & m) | v;
        }
        self
    }

    pub fn get(&self, x: usize, y: usize) -> Entry {
        assert!(x < self.dim());
        assert!(y < self.dim());
        let v = unsafe { self.get_unchecked(x, y) };
        Entry::from_u32(v).unwrap()
    }

    pub unsafe fn get_unchecked(&self, x: usize, y: usize) -> u32 {
        let i = x + y * (self.dim());
        self.get_linear_unchecked(i)
    }

    pub unsafe fn get_linear_unchecked(&self, i: usize) -> u32 {
        let v = self.data.get_unchecked(i / BOARD_WORD_ENTRIES);
        let v = v >> (ENTRY_BITS * (i % BOARD_WORD_ENTRIES));
        v & ENTRY_MASK
    }

    pub fn iter(&self) -> BoardIter {
        BoardIter { board: self, index: 0, bound: self.len() }
    }

    pub fn pretty(&self) -> String {
        self.iter().chunks(self.dim()).into_iter().map(|entries| {
            entries.map(|e| e.pretty()).join("")
        }).join("\n")
    }

    pub fn move_position(&self, m: Move) -> (usize, usize) {
        match m.side {
            Side::North => (m.pos(), 0),
            Side::East => (self.dim() - 1, m.pos()),
            Side::South => (m.pos(), self.dim() - 1),
            Side::West => (0, m.pos()),
        }
    }

    pub fn is_legal_move(&self, m: Move) -> bool {
        if m.pos() >= self.dim() { return false }
        let (x, y) = self.move_position(m);
        let v = unsafe { self.get_unchecked(x, y) };
        v == 0
    }

    pub fn possible_moves(&self) -> PossibleMovesIter {
        SIDES.iter().zip(iter::repeat(self.dim())).flat_map(Side::side_moves)
    }

    pub fn count_legal_moves(&self) -> usize {
        self.possible_moves().filter(|&m| self.is_legal_move(m)).count()
    }

    pub fn nth_legal_move(&self, n: usize) -> Option<Move> {
        self.possible_moves().filter(|&m| self.is_legal_move(m)).nth(n)
    }

    pub fn move_pos_iter(&self, m: Move) -> MovePosIter {
        let (x, y) = self.move_position(m);
        MovePosIter { dim: self.dim(), side: m.side, x: x, y: y }
    }

    pub fn move_pos(&self, m: Move) -> (usize, usize) {
        debug_assert!(self.is_legal_move(m));
        let mut iter = self.move_pos_iter(m);
        let mut pos = iter.next().unwrap();
        for pos1 in iter {
            let (x, y) = pos1;
            let is_empty = unsafe { self.get_unchecked(x, y) == 0 };
            if !is_empty { break; }
            pos = pos1;
        }
        pos
    }

    pub fn is_winning_pos(&self, p: Entry, x: usize, y: usize) -> bool {
        let mut n = 0;
        for y1 in 0..self.dim() {
            let is_match = (y1 == y) || unsafe { p as u32 == self.get_unchecked(x, y1) };
            if is_match { n += 1; if n >= 4 { return true; } } else { n = 0; }
        }
        n = 0;
        for x1 in 0..self.dim() {
            let is_match = (x1 == x) || unsafe { p as u32 == self.get_unchecked(x1, y) };
            if is_match { n += 1; if n >= 4 { return true; } } else { n = 0; }
        }
        n = 0;
        let d = cmp::min(x, y);
        for (x1, y1) in ((x - d)..self.dim()).zip((y - d)..self.dim()) {
            let is_match = (x1 == x && y1 == y) ||
                unsafe { p as u32 == self.get_unchecked(x1, y1) };
            if is_match { n += 1; if n >= 4 { return true; } } else { n = 0; }
        }
        n = 0;
        let d = cmp::min((self.dim() - 1) - x, y);
        for (x1, y1) in (0..(x + d + 1)).rev().zip((y - d)..self.dim()) {
            let is_match = (x1 == x && y1 == y) ||
                unsafe { p as u32 == self.get_unchecked(x1, y1) };
            if is_match { n += 1; if n >= 4 { return true; } } else { n = 0; }
        }
        false
    }

    pub fn is_winning_move(&self, p: Entry, m: Move) -> bool {
        let (x, y) = self.move_pos(m);
        self.is_winning_pos(p, x, y)
    }

    pub fn make_move(self, p: Entry, m: Move) -> Board {
        let (x, y) = self.move_pos(m);
        let self1 = unsafe { self.set_unchecked(x, y, p as u32) };
        self1
    }

    pub fn make_move_check_win(self, p: Entry, m: Move) -> (Board, bool) {
        let (x, y) = self.move_pos(m);
        let self1 = unsafe { self.set_unchecked(x, y, p as u32) };
        let won = self1.is_winning_pos(p, x, y);
        (self1, won)
    }
}

pub type PossibleMovesIter =
    iter::FlatMap<iter::Zip<slice::Iter<'static, Side>, iter::Repeat<usize>>,
                  SideMovesIter, fn((&'static Side, usize)) -> SideMovesIter>;

pub struct BoardIter<'a> {
    board: &'a Board,
    index: usize,
    bound: usize,
}

impl<'a> Iterator for BoardIter<'a> {
    type Item = Entry;

    fn next(&mut self) -> Option<Self::Item> {
        if self.index >= self.bound {
            None
        } else {
            let v = unsafe { self.board.get_linear_unchecked(self.index) };
            self.index += 1;
            Entry::from_u32(v)
        }
    }
}

pub struct MovePosIter {
    dim: usize,
    side: Side,
    x: usize,
    y: usize,
}

impl Iterator for MovePosIter {
    type Item = (usize, usize);

    fn next(&mut self) -> Option<Self::Item> {
        if self.x >= self.dim || self.y >= self.dim {
            None
        } else {
            let result = (self.x, self.y);
            match self.side {
                Side::North => self.y = self.y.wrapping_add(1),
                Side::East => self.x = self.x.wrapping_sub(1),
                Side::South => self.y = self.y.wrapping_sub(1),
                Side::West => self.x = self.x.wrapping_add(1),
            }
            Some(result)
        }
    }
}

trait Player {
    fn entry(&self) -> Entry;
    fn choose_move(&self, b: Board) -> Move;
}

pub struct RandomPlayer {
    p: Entry,
}

impl RandomPlayer {
    pub fn new(p: Entry) -> RandomPlayer {
        RandomPlayer { p: p }
    }
}

impl Player for RandomPlayer {
    fn entry(&self) -> Entry { self.p }

    fn choose_move(&self, b: Board) -> Move {
        let mut n = 0;
        for m in b.possible_moves().filter(|&m| b.is_legal_move(m)) {
            if b.is_winning_move(self.p, m) { return m } else { n += 1 }
        }
        let mut rng = rand::thread_rng();
        let i = rng.gen_range(0, n);
        b.nth_legal_move(i).unwrap()
    }
}

pub struct HumanPlayer {
    p: Entry,
}

impl HumanPlayer {
    pub fn new(p: Entry) -> HumanPlayer {
        HumanPlayer { p: p }
    }
}

impl Player for HumanPlayer {
    fn entry(&self) -> Entry { self.p }

    fn choose_move(&self, _b: Board) -> Move {
        let mut buffer = String::new();
        print!("Side? ");
        io::stdout().flush().unwrap();
        buffer.clear();
        io::stdin().read_line(&mut buffer).unwrap();
        let side = match buffer.trim() {
            "n" => Side::North,
            "e" => Side::East,
            "s" => Side::South,
            "w" => Side::West,
            _ => panic!("invalid side"),
        };
        print!("Position? ");
        io::stdout().flush().unwrap();
        buffer.clear();
        io::stdin().read_line(&mut buffer).unwrap();
        let pos = buffer.trim().parse::<usize>().unwrap();
        Move::new(side, pos)
    }
}

pub struct MCTSPlayer {
    p: Entry
}

impl MCTSPlayer {
    pub fn new(p: Entry) -> MCTSPlayer {
        MCTSPlayer { p: p }
    }
}

impl Player for MCTSPlayer {
    fn entry(&self) -> Entry { self.p }

    fn choose_move(&self, b: Board) -> Move {
        for m in b.possible_moves().filter(|&m| b.is_legal_move(m)) {
            if b.is_winning_move(self.p, m) { return m }
        }
        let mut n = SearchNode::new();
        for _ in 0..100000 { n.explore(self.p, b); }
        println!("{:?}",
                 n.children.as_ref().unwrap().iter()
                 .map(|n| (n.nwin, n.nplay, n.p_win()))
                 .collect::<Vec<_>>());
        n.best_move(b)
    }
}

#[derive(Clone, Debug)]
pub struct SearchNode {
    nwin: u64,
    nplay: u64,
    children: Option<Vec<SearchNode>>,
}

impl SearchNode {
    pub fn new() -> SearchNode {
        SearchNode { nwin: 0, nplay: 0, children: None }
    }

    pub fn best_move(&self, b: Board) -> Move {
        let i = self.children.as_ref().unwrap().iter()
            .enumerate().max_by(|&(_, n1), &(_, n2)| {
                n1.p_win().partial_cmp(&n2.p_win()).unwrap()
            }).map(|(i, _)| i).unwrap();
        b.nth_legal_move(i).unwrap()
    }

    pub fn explore(&mut self, p: Entry, b: Board) -> i32 {
        if self.children.is_none() {
            if self.nwin == std::u64::MAX { return 1; }
            let mut n = 0;
            for m in b.possible_moves().filter(|&m| b.is_legal_move(m)) {
                if b.is_winning_move(p, m) {
                    self.nwin = std::u64::MAX;
                    self.nplay = std::u64::MAX;
                    return 1;
                }
                n += 1;
            }
            self.children = Some(iter::repeat(SearchNode::new()).take(n).collect::<_>());
            self.explore_deepen(p, b, n)
        } else {
            self.explore_tree(p, b)
        }
    }

    pub fn explore_deepen(&mut self, p: Entry, b: Board, n: usize) -> i32 {
        let mut rng = rand::thread_rng();
        let i = rng.gen_range(0, n);
        let b = b.make_move(p, b.nth_legal_move(i).unwrap());
        let p = match p { Entry::Player1 => Entry::Player2, _ => Entry::Player1 };
        let win = -self.children.as_mut().unwrap().get_mut(i).unwrap().explore_random(p, b);
        self.nwin += if win > 0 { 1 } else { 0 };
        self.nplay += 1;
        win
    }

    pub fn explore_random(&mut self, mut p: Entry, mut b: Board) -> i32 {
        let mut rng = rand::thread_rng();
        let mut win = 1;
        'outer: loop {
            let mut n = 0;
            for m in b.possible_moves().filter(|&m| b.is_legal_move(m)) {
                if b.is_winning_move(p, m) { break 'outer; } else { n += 1 }
            }
            if n == 0 { win = 0; break; }
            let i = rng.gen_range(0, n);
            b = b.make_move(p, b.nth_legal_move(i).unwrap());
            p = match p { Entry::Player1 => Entry::Player2, _ => Entry::Player1 };
            win = -win;
        }
        self.nwin += if win > 0 { 1 } else { 0 };
        self.nplay += 1;
        win
    }

    pub fn explore_tree(&mut self, p: Entry, b: Board) -> i32 {
        let children = self.children.as_mut().unwrap();
        let total = children.iter().map(SearchNode::p_win).sum::<f64>();
        if total == 0.0 {
            self.nwin = 0;
            self.nplay = std::u64::MAX;
            return -1;
        }
        let mut rng = rand::thread_rng();
        let x = rng.gen_range(0.0, total);
        let i = children.iter().scan(0.0, |state, n| {
            *state += n.p_win();
            Some(*state)
        }).position(|y| x < y).unwrap();
        let b = b.make_move(p, b.nth_legal_move(i).unwrap());
        let p = match p { Entry::Player1 => Entry::Player2, _ => Entry::Player1 };
        let win = -children.get_mut(i).unwrap().explore(p, b);
        self.nwin += if win > 0 { 1 } else { 0 };
        self.nplay += 1;
        win
    }

    pub fn p_win(&self) -> f64 {
        if self.nplay == std::u64::MAX {
            if self.nwin == 0 { 1.0 } else { 0.0 }
        } else {
            let nwin = self.nwin + PRIOR_SMOOTH;
            let nplay = self.nplay + PRIOR_SMOOTH + PRIOR_SMOOTH;
            (nplay - nwin) as f64 / nplay as f64
        }
    }
}

fn old_main() {
    let mut b = Board::new(10).set(5, 5, Entry::Block).set(9, 2, Entry::Block);
    let players: [Box<Player>; 2] = [Box::new(MCTSPlayer::new(Entry::Player1)),
                                     Box::new(MCTSPlayer::new(Entry::Player2))];
    let mut over = false;

    println!("{}\n--", b.pretty());
    while !over {
        for p in players.iter() {
            let (b1, won) = b.make_move_check_win(p.entry(), p.choose_move(b));
            b = b1;
            println!("{}\n--", b.pretty());
            if won {
                over = true;
                break;
            }
        }
    }
}

fn main() {
    let mut b = board::Board::generate(10, 6);
    let players: [Box<player::Player>; 2] =
        [Box::new(player::HumanPlayer),
         Box::new(player::MCTSPlayer)];
    println!("{}", b);
    for p in players.iter().cycle() {
        let m = p.choose(&b);
        let r = b.make_legal_move(m);
        println!("{}", b);
        match r {
            board::GameState::Ongoing => (),
            board::GameState::Drawn => {
                println!("Drawn.");
                break;
            },
            board::GameState::Won => {
                println!("Won! ({})", b.active());
                break;
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_set_then_get() {
        let b = Board::new(10);
        let b = b.set(5, 7, Entry::Player1);
        assert_eq!(Entry::Empty, b.get(5, 6));
        assert_eq!(Entry::Empty, b.get(6, 8));
        assert_eq!(Entry::Player1, b.get(5, 7));
        let b1 = b.set(5, 7, Entry::Block);
        assert_eq!(Entry::Player1, b.get(5, 7));
        assert_eq!(Entry::Block, b1.get(5, 7));
    }

    #[test]
    fn board_iter() {
        let b = Board::new(2);
        let b = b.set(0, 0, Entry::Block).set(1, 0, Entry::Player1).set(1, 1, Entry::Player2);
        let v = b.iter().collect::<Vec<_>>();
        assert_eq!(vec![Entry::Block, Entry::Player1, Entry::Empty, Entry::Player2], v);
    }

    #[test]
    fn board_test_winning_vert_1() {
        let b = Board::new(10).
            make_move(Entry::Player1, Move::new(Side::North, 4)).
            make_move(Entry::Player1, Move::new(Side::North, 4)).
            make_move(Entry::Player1, Move::new(Side::North, 4)).
            make_move(Entry::Player1, Move::new(Side::North, 4));
        println!("\n{}\n--", b.pretty());
        assert_eq!(false, b.is_winning_pos(Entry::Player1, 3, 6));
        assert_eq!(true, b.is_winning_pos(Entry::Player1, 4, 6));
        assert_eq!(false, b.is_winning_pos(Entry::Player2, 4, 6));
    }

    #[test]
    fn board_test_winning_horiz_1() {
        let b = Board::new(10).
            make_move(Entry::Player1, Move::new(Side::East, 4)).
            make_move(Entry::Player1, Move::new(Side::East, 4)).
            make_move(Entry::Player1, Move::new(Side::East, 4)).
            make_move(Entry::Player1, Move::new(Side::East, 4));
        println!("\n{}\n--", b.pretty());
        assert_eq!(false, b.is_winning_pos(Entry::Player1, 3, 3));
        assert_eq!(true, b.is_winning_pos(Entry::Player1, 3, 4));
        assert_eq!(false, b.is_winning_pos(Entry::Player2, 3, 4));
    }

    #[test]
    fn board_test_winning_diag_1() {
        let b = Board::new(10).
            set(7, 7, Entry::Block).set(6, 6, Entry::Block).
            set(5, 5, Entry::Block).set(4, 4, Entry::Block).
            make_move(Entry::Player1, Move::new(Side::North, 4)).
            make_move(Entry::Player1, Move::new(Side::North, 5)).
            make_move(Entry::Player1, Move::new(Side::North, 6)).
            make_move(Entry::Player1, Move::new(Side::North, 7));
        println!("\n{}\n--", b.pretty());
        assert_eq!(false, b.is_winning_pos(Entry::Player1, 8, 9));
        assert_eq!(true, b.is_winning_pos(Entry::Player1, 7, 6));
        assert_eq!(false, b.is_winning_pos(Entry::Player2, 7, 6));
    }

    #[test]
    fn board_test_winning_diag_2() {
        let b = Board::new(10).
            set(4, 7, Entry::Block).set(5, 6, Entry::Block).
            set(6, 5, Entry::Block).set(7, 4, Entry::Block).
            make_move(Entry::Player1, Move::new(Side::North, 4)).
            make_move(Entry::Player1, Move::new(Side::North, 5)).
            make_move(Entry::Player1, Move::new(Side::North, 6)).
            make_move(Entry::Player1, Move::new(Side::North, 7));
        println!("\n{}\n--", b.pretty());
        assert_eq!(false, b.is_winning_pos(Entry::Player1, 8, 9));
        assert_eq!(true, b.is_winning_pos(Entry::Player1, 7, 3));
        assert_eq!(false, b.is_winning_pos(Entry::Player2, 7, 3));
    }
}
