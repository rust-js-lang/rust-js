// A table of rows, `vec![vec![0; width]; height]`, has rows each as long
// as `width` (#370): `board[r][c]` of `c < width` reads as upstream's
// dynamic programming does. A row changed keeps its check. What Rust does
// with it is what it does natively.

fn unique_paths(width: usize, height: usize) -> u64 {
    let mut board = vec![vec![1; width]; height];
    for r in 1..height {
        for c in 1..width {
            board[r][c] = board[r - 1][c] + board[r][c - 1];
        }
    }
    board[height - 1][width - 1]
}

fn longest_common(a: &[u8], b: &[u8]) -> usize {
    let mut lengths = vec![vec![0; b.len() + 1]; a.len() + 1];
    for (i, &x) in a.iter().enumerate() {
        for (j, &y) in b.iter().enumerate() {
            lengths[i + 1][j + 1] = if x == y {
                lengths[i][j] + 1
            } else {
                lengths[i][j + 1].max(lengths[i + 1][j])
            };
        }
    }
    lengths[a.len()][b.len()]
}

fn shortened(width: usize, height: usize) -> Option<u32> {
    let mut board = vec![vec![1; width]; height];
    board[0].truncate(1);
    let mut sum = 0;
    for r in 0..height {
        for c in 0..width {
            sum += board[r].get(c).copied()?;
        }
    }
    Some(sum)
}

fn main() {
    println!("{} {} {}", unique_paths(3, 3), unique_paths(7, 3), unique_paths(1, 1));
    println!(
        "{} {}",
        longest_common(b"ABCBDAB", b"BDCABA"),
        longest_common(b"", b"abc")
    );
    println!("{:?} {:?}", shortened(1, 3), shortened(2, 2));
}
