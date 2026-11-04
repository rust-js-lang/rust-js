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
// And of a `match` with two arms of `continue`, which MIR joins, each
// commented one's, `if (x == null) { // c\n continue; }` beside a guarded
// arm's or a block's, `default:\n // c\n continue;` after `1 =>
// continue`, each of two blocks', and an outer `match`'s beside an inner
// one's; not one with no comment, the `if` of the rest (#680).
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

pub fn guarded(xs: &[Option<i32>], log: &mut Vec<i32>) {
    for x in xs {
        let n = match x {
            Some(n) if *n > 5 => continue,
            Some(n) => *n,
            // None is skipped.
            None => continue,
        };
        log.push(n);
    }
}

pub fn after_a_block(xs: &[Option<i32>], log: &mut Vec<i32>) {
    for x in xs {
        let n = match x {
            Some(0) => {
                log.push(-1);
                continue;
            }
            Some(n) => *n,
            // None is skipped too.
            None => continue,
        };
        log.push(n);
    }
}

pub fn last_of_two(reason: i32, log: &mut Vec<i32>) {
    for k in 0..reason {
        match k {
            1 => continue,
            3 => log.push(3),
            // The rest are skipped.
            _ => continue,
        }
        log.push(400);
    }
}

pub fn two_blocks(xs: &[Option<i32>], log: &mut Vec<i32>) {
    for x in xs {
        let n = match x {
            Some(0) => {
                // Zero is skipped.
                continue;
            }
            Some(n) => *n,
            None => {
                // None is skipped as well.
                continue;
            }
        };
        log.push(n);
    }
}

pub fn nested_matches(xs: &[Option<Option<i32>>], log: &mut Vec<i32>) {
    for x in xs {
        let n = match x {
            Some(inner) => match inner {
                Some(n) => *n,
                None => continue,
            },
            // An outer None is skipped.
            None => continue,
        };
        log.push(n);
    }
}

pub fn uncommented_joined(xs: &[Option<i32>], log: &mut Vec<i32>) {
    for x in xs {
        let n = match x {
            Some(n) if *n > 5 => continue,
            Some(n) => *n,
            None => continue,
        };
        log.push(n);
    }
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
    guarded(&[Some(2), None, Some(9), Some(4)], &mut log);
    after_a_block(&[Some(0), None, Some(8)], &mut log);
    last_of_two(5, &mut log);
    two_blocks(&[Some(0), Some(6), None, Some(1)], &mut log);
    nested_matches(&[Some(Some(2)), Some(None), None, Some(Some(5))], &mut log);
    uncommented_joined(&[Some(3), None, Some(6)], &mut log);
    println!("{log:?}");
    println!("{}", chars("a b!"));
}
