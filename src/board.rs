use std::cmp;
use std::error;
use std::fmt;
use std::iter;
use std::ops;
use std::slice;

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
            if !is_match { n += 1; if n >= 4 { return true; } } else { n = 0; }
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
        let pos = self.pos + 1;
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

    fn is_legal(&self, b: &Board) -> bool {
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
}
