// A table's `iter_mut()` loop that only writes its rows' cells keeps their
// lengths, so a read of `board[r][c]` after it is known in bounds (#372);
// one that may resize a row, `row.truncate(1)` or a closure that pushes,
// forgets them, and a read past a row's end panics as Rust's does.

fn unique_paths(width: usize, height: usize) -> f64 {
    let mut board = vec![vec![0.0f64; width]; height];
    for (row_index, row) in board.iter_mut().enumerate() {
        for (column_index, cell) in row.iter_mut().enumerate() {
            if row_index == 0 || column_index == 0 {
                *cell = 1.0;
            }
        }
    }
    for row_index in 1..height {
        for column_index in 1..width {
            board[row_index][column_index] =
                board[row_index - 1][column_index] + board[row_index][column_index - 1];
        }
    }
    board[height - 1][width - 1]
}

fn truncated(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1u32; width]; height];
    for row in board.iter_mut() {
        row.truncate(1);
    }
    let mut total = 0;
    for r in 0..height {
        for c in 0..width {
            total += board[r][c];
        }
    }
    total
}

fn pushed(width: usize, height: usize) -> usize {
    let mut board = vec![vec![1u32; width]; height];
    for row in board.iter_mut() {
        let mut grow = || row.pop();
        grow();
    }
    board[0][width - 1] as usize
}

fn replaced(width: usize, height: usize) -> u32 {
    let mut board = vec![vec![1u32; width]; height];
    for row in board.iter_mut() {
        *row = Vec::new();
    }
    board[height - 1][width - 1]
}

fn main() {
    println!("{}", unique_paths(3, 3));
    println!("{}", unique_paths(5, 2));
    println!("{}", std::panic::catch_unwind(|| truncated(3, 2)).is_err());
    println!("{}", std::panic::catch_unwind(|| truncated(1, 2)).unwrap());
    println!("{}", std::panic::catch_unwind(|| pushed(3, 2)).is_err());
    println!("{}", std::panic::catch_unwind(|| replaced(2, 2)).is_err());
}
