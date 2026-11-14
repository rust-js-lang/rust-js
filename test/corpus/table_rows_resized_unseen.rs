// A table of rows, `vec![vec![1; width]; height]`, has its `board[r][c]`
// unchecked under `c < width` only where nothing may resize a row (#1849,
// ADR 0576). A row may resize with no row's `&mut` given to a call:
// through the table's slice, `board.fill(..)`; through an `IterMut` given
// to a function, or held by a struct of the crate's; through what a view
// gives, cleared in a loop, by a closure or `Vec::clear` it's given, or
// collected first; and through a row's `&mut` written whole, swapped or
// taken. Each reads past an emptied row's
// end and panics as Rust does. A `swap` of two rows keeps their lengths,
// as all are as long. A loop that only writes cells of `iter_mut()`'s rows
// keeps them too.
use std::any::Any;
use std::ops::IndexMut;
use std::panic::{self, AssertUnwindSafe};
use std::slice::IterMut;

fn message(payload: Box<dyn Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "another payload".to_string()
    }
}

fn check<T: std::fmt::Debug>(what: &str, f: impl FnOnce() -> T) {
    match panic::catch_unwind(AssertUnwindSafe(f)) {
        Ok(value) => println!("{what}: {value:?}"),
        Err(payload) => println!("{what}: {}", message(payload)),
    }
}

fn first_row(board: &[Vec<u32>], width: usize) -> u32 {
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}

fn shrink(rows: IterMut<'_, Vec<u32>>) {
    for row in rows {
        row.clear();
    }
}

/// Rows a struct of the crate's holds, whose method empties them.
struct Rows<'a>(IterMut<'a, Vec<u32>>);

impl Rows<'_> {
    fn empty(&mut self) {
        for row in &mut self.0 {
            row.clear();
        }
    }
}

fn filled(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    board.fill(Vec::new());
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}

fn viewed(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    shrink(board.iter_mut());
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}

fn held(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    let mut rows = Rows(board.iter_mut());
    rows.empty();
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}

fn replaced(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    board[0] = Vec::new();
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}

fn written(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    *board.index_mut(0) = Vec::new();
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}

fn swapped_out(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    let mut empty = Vec::new();
    std::mem::swap(&mut board[0], &mut empty);
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}

fn taken(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    let row = std::mem::take(&mut board[0]);
    let mut total = row.len() as u32;
    for c in 0..width {
        total += board[0][c];
    }
    total
}

fn rows_swapped(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    board.swap(0, 1);
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}

fn looped(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    for row in board.iter_mut() {
        row.clear();
    }
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}

fn closured(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    board.iter_mut().for_each(|r| r.truncate(0));
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}

fn cleared_each(width: usize, height: usize) -> u32 {
    let mut board: Vec<Vec<u32>> = vec![vec![1; width]; height];
    board.iter_mut().for_each(Vec::clear);
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}

fn collected(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1; width]; height];
    let mut rows: Vec<&mut Vec<u32>> = board.iter_mut().collect();
    rows[0].clear();
    let mut total = 0;
    for c in 0..width {
        total += board[0][c];
    }
    total
}

fn cells_written(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![0; width]; height];
    for row in board.iter_mut() {
        for cell in row.iter_mut() {
            *cell = 2;
        }
    }
    first_row(&board, width)
}

fn main() {
    check("filled", || filled(2, 2));
    check("viewed", || viewed(2, 2));
    check("held", || held(2, 2));
    check("replaced", || replaced(2, 2));
    check("written", || written(2, 2));
    check("swapped out", || swapped_out(2, 2));
    check("taken", || taken(2, 2));
    check("rows swapped", || rows_swapped(2, 2));
    check("looped", || looped(2, 2));
    check("closured", || closured(2, 2));
    check("cleared each", || cleared_each(2, 2));
    check("collected", || collected(2, 2));
    check("cells written", || cells_written(2, 2));
}
