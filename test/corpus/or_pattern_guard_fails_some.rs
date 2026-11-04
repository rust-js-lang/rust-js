// A guarded or-pattern whose guard fails of some of its alternatives,
// `(_, 2, 1, 0) | (_, 2, 0, 2) | (_, _, 1, 0) if b != 2`, compiles (#714):
// MIR tests the guard after each alternative, and inside `if (b === 2)`
// the guard's `if (b !== 2)` is decided, gone, and so is each `if` it
// leaves empty, in the lists around it too, never `if (b === 2) {}`. Each
// input takes the arm native Rust takes.
pub fn guarded(xs: &[u8], b: u8, c: u8, d: u8) -> u8 {
    match (xs, b, c, d) {
        (_, 2, 1, 0) | (_, 2, 0, 2) | (_, _, 1, 0) if b != 2 => 2,
        (_, 1, 1, 1) | ([_, 2], 1, _, 0) | (_, 2, 1, _) if b != 1 => 3,
        _ => 0,
    }
}

pub fn first(xs: &[u8], b: u8, c: u8, d: u8) -> u8 {
    match (xs, b, c, d) {
        ([_], _, 1, 2) if b > 1 => 1,
        (_, 2, 1, 0) | (_, 2, 0, 2) | (_, _, 1, 0) if b != 2 => 2,
        (_, 1, 1, 1) | ([_, 2], 1, _, 0) | (_, 2, 1, _) if b != 1 => 3,
        _ => 0,
    }
}

fn main() {
    let lists: [&[u8]; 4] = [&[], &[1], &[1, 2], &[2, 0]];
    let mut out = Vec::new();
    for xs in lists {
        for b in 0..4 {
            for c in 0..3 {
                for d in 0..4 {
                    out.push(guarded(xs, b, c, d));
                    out.push(first(xs, b, c, d));
                }
            }
        }
    }
    println!("{out:?}");
}
