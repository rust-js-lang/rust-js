// An or-pattern alternative no value reaches, `(_, 1, 1)` after `(_, _, 1)`
// or `(1, _, 0)` after `(1, _, _)`, which rustc only warns of
// (`unreachable_patterns`), compiles (#694): MIR still tests it, and where
// an earlier test decides each of its tests, `c === 1` past `c === 1`'s
// arm, the `if`s left empty go too, never `if (b === 1) {}`. Each input
// takes the arm native Rust takes.
pub fn three(v0: u8, v1: u8, v2: u8) -> u8 {
    match (v0, v1, v2) {
        (_, _, 1) | (_, 1, 1) => 1,
        (_, _, 2) | (1, _, _) => 2,
        (2, _, _) | (1, _, 0) | (0, 1, 1) => 3,
        _ => 0,
    }
}

pub fn first(a: u8, b: u8, c: u8) -> u8 {
    match (a, b, c) {
        (0, 0, 0) => 1,
        (_, _, 1) | (_, 1, 1) => 2,
        (2, _, _) | (2, 2, 2) => 3,
        _ => 4,
    }
}

pub fn present(a: Option<u8>, b: u8, c: u8) -> u8 {
    match (a, b, c) {
        (_, _, 1) | (Some(1), 1, 1) => 1,
        (None, _, 2) | (_, 2, 2) => 2,
        (Some(2), _, _) | (Some(2), 0, 0) | (None, 1, 1) => 3,
        _ => 0,
    }
}

pub fn counted(rows: &[(u8, u8, u8)]) -> Vec<u32> {
    let mut out = Vec::new();
    for &(a, b, c) in rows {
        match (a, b, c) {
            (_, _, 1) | (_, 1, 1) => continue,
            (_, _, 2) | (1, _, _) => out.push(2),
            (2, _, _) | (1, _, 0) | (0, 1, 1) => break,
            _ => {}
        }
        out.push(u32::from(a));
    }
    out
}

fn main() {
    let mut out = Vec::new();
    let mut rows = Vec::new();
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                out.push(three(a, b, c));
                out.push(first(a, b, c));
                rows.push((a, b, c));
                for x in [None, Some(a)] {
                    out.push(present(x, b, c));
                }
            }
        }
    }
    println!("{out:?}");
    println!("{:?}", counted(&rows));
    println!("{:?}", counted(&[(0, 0, 1), (0, 1, 1), (3, 3, 2), (0, 0, 0), (2, 0, 0), (3, 3, 3)]));
}
