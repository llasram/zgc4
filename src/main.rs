#[macro_use] extern crate enum_primitive;
extern crate itertools;
extern crate num_traits;
extern crate smallvec;

use num_traits::FromPrimitive;
use itertools::Itertools;

const ENTRY_BITS: usize = 2;
const ENTRY_MASK: u32 = !(!0u32 << ENTRY_BITS);
const BOARD_WORD_BITS: usize = 32;
const BOARD_WORD_ENTRIES: usize = BOARD_WORD_BITS / ENTRY_BITS;
const BOARD_WORDS: usize = 16;

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
}

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

fn main() {
    let b = Board::new(10).set(5, 5, Entry::Block).set(9, 2, Entry::Block);
    println!("{}", b.pretty());
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
}
