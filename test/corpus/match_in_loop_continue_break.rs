// A `match` in a loop whose arms `continue`, push, `break` and do
// nothing, a statement after it: the chain of its arms, `if (c === 1) {
// continue; } if (a === 1 || c === 2) { out.push(2); } else if (a === 2) {
// break; } out.push(a);`, no labeled block of MIR's (#696). Of an arm
// after `c === 1`'s, its test of `c` says only what that one's doesn't,
// `c !== 2` of `c !== 1 && c !== 2`, and so goes, `c === 2` its other
// alternative.

fn counted(rows: &[(u8, u8, u8)]) -> Vec<u32> {
    let mut out = Vec::new();
    for &(a, b, c) in rows {
        match (a, b, c) {
            (_, _, 1) => continue,
            (_, _, 2) | (1, _, _) => out.push(2),
            (2, _, _) => break,
            _ => {}
        }
        out.push(u32::from(a) * 10 + u32::from(b));
    }
    out
}

fn main() {
    let mut rows = Vec::new();
    for a in 0..3 {
        for b in 0..2 {
            for c in 0..4 {
                rows.push((a, b, c));
            }
        }
    }
    println!("{:?}", counted(&rows));
    println!("{:?}", counted(&rows[8..]));
    println!("{:?}", counted(&[(1, 0, 0), (0, 1, 3), (2, 0, 2), (3, 3, 3)]));
}
