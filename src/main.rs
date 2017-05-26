#[macro_use] extern crate enum_primitive;
extern crate num_traits;
extern crate smallvec;

use num_traits::FromPrimitive;

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

#[derive(Clone, Copy, Debug)]
pub struct Board {
    size: u32,
    data: [u32; BOARD_WORDS],
}

impl Board {
    pub fn new(size: usize) -> Board {
        Board { size: size as u32, data: [0; BOARD_WORDS] }
    }

    pub fn set(self, x: usize, y: usize, v: Entry) -> Board {
        assert!(x < self.size as usize);
        assert!(y < self.size as usize);
        unsafe { self.set_unchecked(x, y, v as u32) }
    }

    pub unsafe fn set_unchecked(mut self, x: usize, y: usize, v: u32) -> Board {
        let i = x * (self.size as usize) + y;
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
        assert!(x < self.size as usize);
        assert!(y < self.size as usize);
        let v = unsafe { self.get_unchecked(x, y) };
        Entry::from_u32(v).unwrap()
    }

    pub unsafe fn get_unchecked(&self, x: usize, y: usize) -> u32 {
        let i = x * (self.size as usize) + y;
        let v = self.data.get_unchecked(i / BOARD_WORD_ENTRIES);
        let v = v >> (ENTRY_BITS * (i % BOARD_WORD_ENTRIES));
        v & ENTRY_MASK
    }
}

fn main() {
    println!("Hello, world!");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_then_get() {
        let b = Board::new(10);
        let b = b.set(5, 7, Entry::Player1);
        assert_eq!(Entry::Empty, b.get(5, 6));
        assert_eq!(Entry::Empty, b.get(6, 8));
        assert_eq!(Entry::Player1, b.get(5, 7));
        let b1 = b.set(5, 7, Entry::Block);
        assert_eq!(Entry::Player1, b.get(5, 7));
        assert_eq!(Entry::Block, b1.get(5, 7));
    }
}
