use std::cmp;
use std::error;
use std::fmt;
use std::iter;

use itertools::Itertools;
use rand;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    IllegalMove(Move),
}

pub type Result<T> = ::std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::IllegalMove(ref m) => write!(f, "Error: {:?}: illegal move", m),
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::IllegalMove(..) => "Illegal move",
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::IllegalMove(..) => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    Empty,
    Block,
    Player1,
    Player2,
}

impl fmt::Display for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Entry::Empty => write!(f, "  "),
            Entry::Block => write!(f, "█▋"),
            Entry::Player1 => write!(f, "●1"),
            Entry::Player2 => write!(f, "○2"),
        }
    }
}

impl Entry {
    pub fn is_empty(self) -> bool { self == Entry::Empty }

    pub fn flip(self) -> Entry {
        match self {
            Entry::Empty => Entry::Block,
            Entry::Block => Entry::Empty,
            Entry::Player1 => Entry::Player2,
            Entry::Player2 => Entry::Player1,
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

impl Side {
    pub fn succ(self) -> Option<Self> {
        match self {
            Side::North => Some(Side::East),
            Side::East => Some(Side::South),
            Side::South => Some(Side::West),
            Side::West => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameState {
    Ongoing,
    Drawn,
    Won,
}

#[derive(Clone, Debug)]
pub struct Board {
    size: usize,
    active: Entry,
    nlegal: usize,
    state: GameState,
    data: Box<[Entry]>,
}

impl Board {
    pub fn new(size: usize) -> Board {
        let len = size * size;
        let active = Entry::Player1;
        let nlegal = size * 4;
        let state = GameState::Ongoing;
        let data = iter::repeat(Entry::Empty).take(len).collect::<Vec<_>>().into_boxed_slice();
        Board { size, active, nlegal, state, data }
    }

    pub fn generate(size: usize, filled: usize) -> Board {
        let mut b = Board::new(size);
        let mut rng = rand::thread_rng();
        for i in rand::sample(&mut rng, 0..b.data.len(), filled).into_iter() {
            let (row, col) = b.pos_for(i);
            b.set(row, col, Entry::Block);
        }
        b
    }

    pub fn active(&self) -> Entry { self.active }

    fn pos_for(&self, index: usize) -> (usize, usize) {
        let row = index / self.size;
        let col = index % self.size;
        (row, col)
    }

    fn index_for(&self, row: usize, col: usize) -> usize {
        row * self.size + col
    }

    pub fn get(&self, row: usize, col: usize) -> Option<Entry> {
        self.data.get(self.index_for(row, col)).cloned()
    }

    pub unsafe fn get_unchecked(&self, row: usize, col: usize) -> Entry {
        *self.data.get_unchecked(self.index_for(row, col))
    }

    pub fn set(&mut self, row: usize, col: usize, entry: Entry) -> () {
        let i = self.index_for(row, col);
        let e = self.data.get_mut(i).unwrap();
        if e.is_empty() && !entry.is_empty() {
            if row == 0 || row == (self.size - 1) { self.nlegal -= 1; }
            if col == 0 || col == (self.size - 1) { self.nlegal -= 1; }
        }
        *e = entry;
    }

    #[inline]
    fn is_winning_horiz(&self, row: usize, col: usize) -> bool {
        let mut n = 0;
        for col1 in 0..self.size {
            let is_this = col1 == col;
            let is_active = self.active == unsafe { self.get_unchecked(row, col1) };
            let is_match = is_this || is_active;
            if is_match { n += 1; if n >= 4 { return true; } } else { n = 0; }
        }
        false
    }

    #[inline]
    fn is_winning_vert(&self, row: usize, col: usize) -> bool {
        let mut n = 0;
        for row1 in 0..self.size {
            let is_this = row1 == row;
            let is_active = self.active == unsafe { self.get_unchecked(row1, col) };
            let is_match = is_this || is_active;
            if is_match { n += 1; if n >= 4 { return true; } } else { n = 0; }
        }
        false
    }

    #[inline]
    fn is_winning_diag_nw_se(&self, row: usize, col: usize) -> bool {
        let mut n = 0;
        let d = cmp::min(row, col);
        for (row1, col1) in ((row - d)..self.size).zip((col - d)..self.size) {
            let is_this = row1 == row && col1 == col;
            let is_active = self.active == unsafe { self.get_unchecked(row1, col1) };
            let is_match = is_this || is_active;
            if is_match { n += 1; if n >= 4 { return true; } } else { n = 0; }
        }
        false
    }

    #[inline]
    fn is_winning_diag_sw_ne(&self, row: usize, col: usize) -> bool {
        let mut n = 0;
        let d = cmp::min(self.size - row - 1, col);
        for (row1, col1) in (0..(row + d + 1)).rev().zip((col - d)..self.size) {
            let is_this = row1 == row && col1 == col;
            let is_active = self.active == unsafe { self.get_unchecked(row1, col1) };
            let is_match = is_this || is_active;
            if is_match { n += 1; if n >= 4 { return true; } } else { n = 0; }
        }
        false
    }

    fn is_winning(&self, row: usize, col: usize) -> bool {
        self.is_winning_horiz(row, col)
            || self.is_winning_vert(row, col)
            || self.is_winning_diag_nw_se(row, col)
            || self.is_winning_diag_sw_ne(row, col)
    }

    pub fn legal_moves_iter(&self) -> LegalMovesIter {
        LegalMovesIter { board: self, base: Some(Move::new(Side::North, 0)) }
    }

    pub fn make_move(&mut self, m: Move) -> Result<GameState> {
        m.annotated(self).map(|m| self.make_legal_move(m)).ok_or(Error::IllegalMove(m))
    }

    pub fn make_legal_move(&mut self, m: LegalMove) -> GameState {
        debug_assert!(self.state == GameState::Ongoing);
        let active = self.active;
        self.set(m.row, m.col, active);
        if m.is_winning {
            self.state = GameState::Won;
        } else if self.nlegal == 0 {
            self.state = GameState::Drawn;
        } else {
            self.active = active.flip();
        }
        self.state
    }

    pub fn pass(&mut self) -> () {
        debug_assert!(self.state == GameState::Ongoing);
        self.active = self.active.flip();
    }
}

impl fmt::Display for Board {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "   ")?;
        for i in 0..self.size { write!(f, "{: >2}", i)?; }
        write!(f, "\n")?;
        for (i, entries) in self.data.iter().chunks(self.size).into_iter().enumerate() {
            write!(f, "{: >2} ", i)?;
            for e in entries { write!(f, "{}", e)?; }
            write!(f, "\n")?;
        }
        Ok(())
    }
}

pub struct LegalMovesIter<'a> {
    board: &'a Board,
    base: Option<Move>,
}

impl<'a> Iterator for LegalMovesIter<'a> {
    type Item = LegalMove;

    fn next(&mut self) -> Option<Self::Item> {
        self.base.and_then(|base| {
            self.base = base.succ(self.board);
            base.annotated(self.board).or_else(|| self.next())
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move  {
    side: Side,
    pos: usize,
}

impl Move {
    pub fn new(side: Side, pos: usize) -> Move {
        Move { side, pos }
    }

    fn succ(&self, b: &Board) -> Option<Move> {
        let pos = self.pos.wrapping_add(1);
        if pos < b.size {
            Some(Move::new(self.side, pos))
        } else {
            self.side.succ().map(|side| Move::new(side, 0))
        }
    }

    fn origin(&self, b: &Board) -> (usize, usize) {
        match self.side {
            Side::North => (0, self.pos),
            Side::East => (self.pos, b.size - 1),
            Side::South => (b.size - 1, self.pos),
            Side::West => (self.pos, 0),
        }
    }

    pub fn is_legal(&self, b: &Board) -> bool {
        let (row, col) = self.origin(b);
        b.get(row, col).map_or(false, Entry::is_empty)
    }

    fn iter<'a>(&self, b: &'a Board) -> MoveVectorIter<'a> {
        let (row, col) = self.origin(b);
        MoveVectorIter { board: b, side: self.side, row, col }
    }

    fn target(&self, b: &Board) -> Option<(usize, usize)> {
        self.iter(b).take_while(|&(_, _, entry)| entry.is_empty())
            .last().map(|(row, col, _)| (row, col))
    }

    pub fn annotated(&self, b: &Board) -> Option<LegalMove> {
        self.target(b).map(|(row, col)| {
            let is_winning = b.is_winning(row, col);
            LegalMove { base: *self, row, col, is_winning }
        })
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MoveVectorIter<'a> {
    board: &'a Board,
    side: Side,
    row: usize,
    col: usize,
}

impl<'a> Iterator for MoveVectorIter<'a> {
    type Item = (usize, usize, Entry);

    fn next(&mut self) -> Option<Self::Item> {
        if self.row >= self.board.size || self.col >= self.board.size {
            None
        } else {
            let entry = unsafe { self.board.get_unchecked(self.row, self.col) };
            let result = (self.row, self.col, entry);
            match self.side {
                Side::North => self.row = self.row.wrapping_add(1),
                Side::East => self.col = self.col.wrapping_sub(1),
                Side::South => self.row = self.row.wrapping_sub(1),
                Side::West => self.col = self.col.wrapping_add(1),
            }
            Some(result)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LegalMove {
    base: Move,
    row: usize,
    col: usize,
    is_winning: bool,
}

impl LegalMove {
    pub fn is_winning(&self) -> bool { self.is_winning }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entry_test_flip() {
        assert_eq!(Entry::Block, Entry::Empty.flip());
        assert_eq!(Entry::Player2, Entry::Player1.flip());
        assert_eq!(Entry::Player1, Entry::Player2.flip());
    }

    #[test]
    fn entry_test_is_empty() {
        assert!(Entry::Empty.is_empty());
        assert!(!Entry::Block.is_empty());
        assert!(!Entry::Player1.is_empty());
        assert!(!Entry::Player2.is_empty());
    }

    #[test]
    fn side_succ() {
        assert_eq!(Some(Side::East), Side::North.succ());
        assert_eq!(Some(Side::South), Side::East.succ());
        assert_eq!(Some(Side::West), Side::South.succ());
        assert_eq!(None, Side::West.succ());
    }

    #[test]
    fn board_set_then_get() {
        let mut b = Board::new(10);
        b.set(5, 7, Entry::Player1);
        assert_eq!(Some(Entry::Empty), b.get(5, 6));
        assert_eq!(Some(Entry::Empty), b.get(6, 8));
        assert_eq!(Some(Entry::Player1), b.get(5, 7));
        b.set(5, 7, Entry::Block);
        assert_eq!(Some(Entry::Block), b.get(5, 7));
    }

    #[test]
    fn board_winning_vert() {
        let mut b = Board::new(10);
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::North, 4))); b.pass();
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::North, 4))); b.pass();
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::North, 4))); b.pass();
        assert_eq!(Ok(GameState::Won), b.make_move(Move::new(Side::North, 4)));
    }

    #[test]
    fn board_winning_horiz() {
        let mut b = Board::new(10);
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::East, 4))); b.pass();
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::East, 4))); b.pass();
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::East, 4))); b.pass();
        assert_eq!(Ok(GameState::Won), b.make_move(Move::new(Side::East, 4)));
    }

    #[test]
    fn board_winning_diag_nw_se() {
        let mut b = Board::new(10);
        b.set(4, 4, Entry::Block);
        b.set(5, 5, Entry::Block);
        b.set(6, 6, Entry::Block);
        b.set(7, 7, Entry::Block);
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::North, 4))); b.pass();
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::North, 5))); b.pass();
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::North, 6))); b.pass();
        assert_eq!(Ok(GameState::Won), b.make_move(Move::new(Side::North, 7)));
    }

    #[test]
    fn board_winning_diag_sw_ne_1() {
        let mut b = Board::new(10);
        b.set(4, 7, Entry::Block);
        b.set(5, 6, Entry::Block);
        b.set(6, 5, Entry::Block);
        b.set(7, 4, Entry::Block);
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::North, 4))); b.pass();
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::North, 5))); b.pass();
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::North, 6))); b.pass();
        assert_eq!(Ok(GameState::Won), b.make_move(Move::new(Side::North, 7)));
    }

    #[test]
    fn board_winning_diag_sw_ne_2() {
        let mut b = Board::new(10);
        b.set(4, 0, Entry::Block);
        b.set(3, 1, Entry::Block);
        b.set(2, 2, Entry::Block);
        b.set(1, 3, Entry::Block);
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::North, 0))); b.pass();
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::North, 1))); b.pass();
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::North, 2))); b.pass();
        assert_eq!(Ok(GameState::Won), b.make_move(Move::new(Side::North, 3)));
    }

    #[test]
    fn board_winning_diag_sw_ne_3() {
        let mut b = Board::new(10);
        b.set(4, 6, Entry::Block);
        b.set(3, 7, Entry::Block);
        b.set(2, 8, Entry::Block);
        b.set(1, 9, Entry::Block);
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::North, 6))); b.pass();
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::North, 7))); b.pass();
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::North, 8))); b.pass();
        assert_eq!(Ok(GameState::Won), b.make_move(Move::new(Side::North, 9)));
    }

    #[test]
    fn board_winning_diag_sw_ne_4() {
        let mut b = Board::new(10);
        b.set(8, 6, Entry::Block);
        b.set(7, 7, Entry::Block);
        b.set(6, 8, Entry::Block);
        b.set(5, 9, Entry::Block);
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::South, 6))); b.pass();
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::South, 7))); b.pass();
        assert_eq!(Ok(GameState::Ongoing), b.make_move(Move::new(Side::South, 9))); b.pass();
        assert_eq!(Ok(GameState::Won), b.make_move(Move::new(Side::South, 8)));
    }

    #[test]
    fn board_legal_moves_iter() {
        let mut b = Board::new(2);
        assert_eq!(b.nlegal, b.legal_moves_iter().count());
        b.set(0, 0, Entry::Block);
        assert_eq!(b.nlegal, b.legal_moves_iter().count());
        b.set(1, 1, Entry::Block);
        assert_eq!(b.nlegal, b.legal_moves_iter().count());
        b.set(0, 1, Entry::Block);
        assert_eq!(b.nlegal, b.legal_moves_iter().count());
        b.set(1, 0, Entry::Block);
        assert_eq!(0, b.nlegal);
        assert_eq!(0, b.legal_moves_iter().count());
    }

    #[test]
    fn move_is_legal() {
        let mut b = Board::new(2);
        b.set(0, 0, Entry::Block);
        assert!(Move::new(Side::North, 1).is_legal(&b));
        assert!(!Move::new(Side::North, 0).is_legal(&b));
        assert!(!Move::new(Side::West, 0).is_legal(&b));
        assert!(Move::new(Side::West, 1).is_legal(&b));
    }

    #[test]
    fn legal_move_is_winning() {
        let mut b = Board::new(4);
        let m = Move::new(Side::North, 0);
        b.make_move(m).ok(); b.pass();
        b.make_move(m).ok(); b.pass();
        b.make_move(m).ok(); b.pass();
        assert_eq!(Some(true), m.annotated(&b).as_ref().map(LegalMove::is_winning));
    }
}
