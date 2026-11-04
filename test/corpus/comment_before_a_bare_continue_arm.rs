// A comment above an arm that's only a `continue`, `// Skipped.\n 1 =>
// continue,`, keeps that `continue` where it's written, the comment
// before it, as an arm whose block holds them does (#610, the user's
// decision): in its `case`, `case 1:\n // Skipped.\n continue;`, as
// React DOM's `case 'ref':\n // Noop\n continue;`; in the `if` of its
// pattern, `if (k === 3) { // c\n continue; }`; and of one MIR tests where
// the `match` is, `None => continue` after `Some(n) => *n`, `if (x == null)
// { // c\n continue; }`, its block's too; and of a last `_`, the `else` of
// the arm before's `if`, `} else { // c\n continue; }`, as of a last arm
// that's a block of only a `continue`, its comment in the block or above
// the arm (#666). One with no comment is the `if` of the rest, as before.
pub fn switched(reason: i32, log: &mut Vec<i32>) {
    for k in 0..reason {
        match k {
            0 => log.push(0),
            // Skipped.
            1 => continue,
            _ => log.push(k),
        }
        log.push(100);
    }
}

pub fn tested(reason: i32, log: &mut Vec<i32>) {
    for k in 0..reason {
        match k {
            // Three is skipped.
            3 => continue,
            _ => log.push(k),
        }
        log.push(200);
    }
}

pub fn two(reason: i32, log: &mut Vec<i32>) {
    for k in 0..reason {
        match k {
            // One is skipped.
            1 => continue,
            2 => log.push(-2),
            // Three is skipped.
            3 => continue,
            _ => log.push(k),
        }
        log.push(500);
    }
}

pub fn options_two(xs: &[Option<i32>], log: &mut Vec<i32>) {
    for x in xs {
        let n = match x {
            // Zero is skipped.
            Some(0) => continue,
            Some(n) => *n,
            None => continue,
        };
        log.push(n);
    }
}

pub fn uncommented(reason: i32, log: &mut Vec<i32>) {
    for k in 0..reason {
        match k {
            3 => continue,
            _ => log.push(k),
        }
        log.push(300);
    }
}

pub fn later(reason: i32, log: &mut Vec<i32>) {
    for k in 0..reason {
        match k {
            3 => log.push(3),
            // Others are skipped.
            _ => continue,
        }
        log.push(400);
    }
}

pub fn options(xs: &[Option<i32>], log: &mut Vec<i32>) {
    for x in xs {
        let n = match x {
            Some(n) => *n,
            // None is skipped.
            None => continue,
        };
        log.push(n);
    }
}

pub fn blocked(xs: &[Option<i32>], log: &mut Vec<i32>) {
    for x in xs {
        let n = match x {
            Some(n) => *n,
            None => {
                // None is skipped here too.
                continue;
            }
        };
        log.push(n + 1);
    }
}

pub fn last_block(xs: &[Option<i32>], log: &mut Vec<i32>) {
    for x in xs {
        match x {
            Some(n) => log.push(*n),
            _ => {
                // Nothing is skipped.
                continue;
            }
        }
        log.push(600);
    }
}

pub fn above_block(reason: i32, log: &mut Vec<i32>) {
    for k in 0..reason {
        match k {
            3 => log.push(3),
            // Others are skipped, a block's.
            _ => {
                continue;
            }
        }
        log.push(700);
    }
}

pub fn uncommented_block(reason: i32, log: &mut Vec<i32>) {
    for k in 0..reason {
        match k {
            3 => log.push(3),
            _ => {
                continue;
            }
        }
        log.push(800);
    }
}

pub fn chars(text: &str) -> String {
    let mut out = String::new();
    for c in text.chars() {
        match c {
            // Spaces are skipped.
            ' ' => continue,
            'a'..='z' => out.push(c.to_ascii_uppercase()),
            _ => out.push(c),
        }
        out.push('.');
    }
    out
}

fn main() {
    let mut log = Vec::new();
    switched(4, &mut log);
    tested(5, &mut log);
    two(5, &mut log);
    options_two(&[Some(0), None, Some(7)], &mut log);
    uncommented(5, &mut log);
    later(5, &mut log);
    options(&[Some(1), None, Some(3)], &mut log);
    blocked(&[None, Some(5)], &mut log);
    last_block(&[Some(4), None, Some(6)], &mut log);
    above_block(5, &mut log);
    uncommented_block(5, &mut log);
    println!("{log:?}");
    println!("{}", chars("a b!"));
}
