// A `match` of constants an arm of which breaks the loop around it (#389,
// the user's decision): a `switch` left by a `break;` after it where the
// loop's turn ends at it, each case that goes on `continue;`; an `if` chain
// where the turn goes on past it; the program's own label kept.
fn first_stop(codes: &[u8]) -> usize {
    let mut i = 0;
    while i < codes.len() {
        match codes[i] {
            b'.' | b'!' => break,
            b' ' => {}
            _ => i += 1,
        }
        i += 1;
    }
    i
}

fn framed(codes: &[u8]) -> u32 {
    let mut i = 0;
    let mut n = 0;
    while i < codes.len() {
        i += 1;
        match codes[i - 1] {
            b'.' | b'!' => break,
            b' ' => n += 2,
            b'#' => continue,
            b'-' => n += 1,
            _ => {}
        }
    }
    n
}

fn labeled(rows: &[&[u8]]) -> u32 {
    let mut n = 0;
    'rows: for row in rows {
        for &b in *row {
            match b {
                0 => break 'rows,
                1 => n += 1,
                _ => n += 10,
            }
            n += 100;
        }
    }
    n
}

fn main() {
    for s in ["", " .", ".", "a", "ab c.d", "xyz"] {
        println!("{:?} {} {}", s, first_stop(s.as_bytes()), framed(s.as_bytes()));
    }
    println!("{}", framed(b"a -#.--"));
    println!("{} {}", labeled(&[&[1, 2], &[0, 1]]), labeled(&[&[2]]));
}
