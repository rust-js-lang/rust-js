// A `match` of constants an arm of which breaks the loop around it (#389,
// the user's decision): a `switch` left by a `break;` after it where the
// loop's turn ends at it, each case that goes on `continue;`; an `if` chain
// where the turn goes on past it; the program's own label kept. So too
// where MIR leaves the loop through a block of its own, an arm's `break`
// and a nested `match`'s (`skipped`, #389c): `break;`, no `bb22:`; and
// where the block's `break` is in a loop of its own, whose way out MIR
// shares with the outer's (`rows_until`): the inner's `if k == 0 {
// break; }` after it, as written, no `bb25:`; and a name the turn
// declares again after the `match` is told apart (`shadowed`).
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

fn skipped(codes: &[u8]) -> (usize, u32) {
    let mut i = 0;
    let mut n = 0;
    while i < codes.len() {
        match codes[i] {
            b'a' => break,
            b'b' => {
                i += 2;
                continue;
            }
            b'd' => match n {
                0 => break,
                1..10 => n += 100,
                _ => {}
            },
            _ => {}
        }
        i += 1;
        n += 1;
    }
    (i, n)
}

fn rows_until(rows: &[&[u8]]) -> u32 {
    let mut n = 0;
    let mut r = 0;
    while r < rows.len() {
        let row = rows[r];
        match row.len() {
            0 => break,
            2 => {
                let mut k = 0;
                while k < row.len() {
                    if row[k] == 7 {
                        break;
                    }
                    k += 1;
                }
                if k == 0 {
                    break;
                }
                n += k as u32;
            }
            _ => {}
        }
        let row = r * 2;
        n += row as u32;
        r += 1;
    }
    n
}

fn shadowed(codes: &[u8]) -> u32 {
    let mut n = 0;
    let mut i = 0;
    while i < codes.len() {
        let c = codes[i];
        match c {
            b'a' => break,
            b'b' => n += 5,
            _ => {}
        }
        let c = format!("{c}");
        n += c.len() as u32;
        i += 1;
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
    for s in ["", "a", "xbxa", "d", "xd", "xxdxd", "bbbb", "xyz"] {
        println!("{:?} {:?}", s, skipped(s.as_bytes()));
    }
    let rows: [&[u8]; 4] = [&[1], &[1, 2], &[3, 7], &[7, 1]];
    println!("{} {} {}", rows_until(&rows), rows_until(&rows[..2]), rows_until(&[&[1], &[]]));
    println!("{} {} {}", shadowed(b"xbyb"), shadowed(b"cab"), shadowed(b""));
    println!("{} {}", labeled(&[&[1, 2], &[0, 1]]), labeled(&[&[2]]));
}
