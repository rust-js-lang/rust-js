//! A consumer that changes in place the copies a library's exported
//! functions hand out, which their private callees make.
use dep::Board;

pub fn main() {
    let board = Board::new();
    let mut top = board.top();
    top.n += 10;
    let mut bottom = board.bottom();
    bottom.n *= 7;
    println!("{top:?} {bottom:?} {:?}", board.tallies());
}
