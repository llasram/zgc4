extern crate smallvec;

const ENTRY_BITS: usize = 2;
const BOARD_WORD_BITS: usize = 32;
const BOARD_WORD_ENTRIES: usize = BOARD_WORD_BITS / ENTRY_BITS;
const BOARD_WORDS: usize = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    Empty,
    Block,
    Player1,
    Player2,
}
use Entry::*;

impl Entry {
    pub fn from_u32(x: u32) -> Entry {
        match x {
            0 => Empty,
            1 => Block,
            2 => Player1,
            3 => Player2,
            _ => panic!("invalid entry bits")
        }
    }

    pub fn to_u32(&self) -> u32 {
        match *self {
            Empty => 0,
            Block => 1,
            Player1 => 2,
            Player2 => 3,
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Action(usize);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ActionSet {
    bits: u64,
}

impl ActionSet {
    pub fn new() -> ActionSet {
        ActionSet { bits: 0 }
    }

    pub fn set(&self, index: usize) -> ActionSet {
        ActionSet { bits: self.bits | (1 << index) }
    }

    pub fn len(&self) -> usize {
        self.bits.count_ones() as usize
    }

    pub fn iter(&self) -> ActionSetIter {
        ActionSetIter { index: 0, bits: self.bits }
    }
}

pub struct ActionSetIter {
    index: usize,
    bits: u64,
}

impl Iterator for ActionSetIter {
    type Item = Action;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bits == 0 {
            None
        } else {
            let z = self.bits.trailing_zeros() as usize;
            let a = Action(self.index + z);
            self.index += z + 1;
            self.bits >>= z + 1;
            Some(a)
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct State {
    pub player: u16,
    pub bsize: u16,
    pub board: [u32; BOARD_WORDS],
}

impl State {
    fn entry_at_linear(&self, index: usize) -> Entry {
        let wi = index / BOARD_WORD_ENTRIES;
        let bi = index % BOARD_WORD_ENTRIES;
        Entry::from_u32((self.board[wi] >> bi) & 3)
    }

    pub fn entry_at(&self, row: usize, col: usize) -> Entry {
        self.entry_at_linear(self.bsize as usize * row + col)
    }

    fn place_at_linear(&mut self, entry: Entry, index: usize) {
        let wi = index / BOARD_WORD_ENTRIES;
        let bi = index % BOARD_WORD_ENTRIES;
        self.board[wi] = (self.board[wi] & !(3 << bi)) | (entry.to_u32() << bi);
    }

    fn place_at(&mut self, entry: Entry, row: usize, col: usize) {
        let index = self.bsize as usize * row + col;
        self.place_at_linear(entry, index);
    }

    pub fn transition(&self, action: Action) -> Option<State> {
        let side = action.0 / self.bsize as usize;
        let index = action.0 % self.bsize as usize;
        let (mut row, rstep, mut col, cstep) = match side {
            0 => (0, 1usize, index, 0usize),
            1 => (index, 0, self.bsize as usize - 1, -1isize as usize),
            2 => (self.bsize as usize - 1, -1isize as usize, index, 0),
            3 => (index, 0usize, 0, 1usize),
            _ => panic!("invalid action")
        };
        if self.entry_at(row, col) != Empty { return None; }
        for _ in 0..(self.bsize - 1) as usize {
            let row1 = row.wrapping_add(rstep);
            let col1 = col.wrapping_add(cstep);
            if self.entry_at(row1, col1) != Empty { break; }
            row = row1;
            col = col1;
        }
        let mut board = *self;
        let entry = match self.player { 0 => Player1, _ => Player2};
        board.place_at(entry, row, col);
        Some(board)
    }
}

fn main() {
    println!("Hello, world!");
}
