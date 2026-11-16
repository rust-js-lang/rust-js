// A loop's label of the word of the loop around it, which Rust allows,
// with a lint, where JS refuses a label inside one of its name: each
// `break 'a` leaves its innermost `'a`, the inner its own, the outer its
// own, so the two are told apart ("names are ids", ADR 1150).
#[allow(unused_labels, label_break_value)]
fn steps(n: u32) -> Vec<u32> {
    let mut seen = Vec::new();
    let mut i = 0;
    #[allow(clippy::never_loop)]
    'a: loop {
        'a: loop {
            i += 1;
            seen.push(i);
            if i % 3 == 0 {
                break 'a;
            }
        }
        seen.push(100 + i);
        if i >= n {
            break 'a;
        }
    }
    seen
}

// `continue 'a` inside the inner `'a`, its own loop's next turn; then a
// `break 'a` of the outer, past its own `'a`.
#[allow(unused_labels)]
fn skipped(rows: &[Vec<u32>]) -> Vec<u32> {
    let mut kept = Vec::new();
    'a: for row in rows {
        'a: for &x in row {
            if x % 2 == 0 {
                continue 'a;
            }
            kept.push(x);
        }
        if kept.len() > 3 {
            break 'a;
        }
        kept.push(0);
    }
    kept
}

// A block labeled `'a`, a loop inside it labeled `'a` too: the loop's
// `break 'a` leaves the loop, the block's leaves the block.
#[allow(unused_labels)]
fn summed(n: u32) -> u32 {
    let mut total = 0;
    'a: {
        for x in 0..n {
            'a: loop {
                total += x;
                if total > 4 {
                    break 'a;
                }
                total += 1;
            }
            if total > 20 {
                break 'a;
            }
        }
        total += 1000;
    }
    total
}

fn main() {
    println!("{:?}", steps(7));
    println!("{:?}", skipped(&[vec![1, 2, 3], vec![4, 5], vec![7, 9, 11]]));
    println!("{} {} {}", summed(0), summed(3), summed(9));
}
