// A comment above a last `_ => {}` changes neither the `switch` nor where
// what follows the `match` is: each case's `break;`, one `return
// finish(log)` after the `switch`, as React's
// attemptEarlyBailoutIfNoScheduledUpdate writes its `// Fallthrough`
// (rd701). The mark of the comment is read past while the `switch` is made.
const ENABLE_A: bool = true;
const ENABLE_B: bool = false;

fn push(log: &mut Vec<u32>, n: u32) {
    log.push(n);
}

fn finish(log: &mut Vec<u32>) -> u32 {
    log.len() as u32
}

pub fn bail(tag: u32, log: &mut Vec<u32>) -> u32 {
    match tag {
        0 => {
            push(log, 0);
        }
        1 | 2 => {
            push(log, 1);
        }
        3 => {
            if log.is_empty() {
                return 7;
            }
        }
        4 if ENABLE_A => {
            push(log, 4);
        }
        4 | 5 if ENABLE_B => {
            return 9;
        }
        // Fallthrough
        _ => {}
    }
    finish(log)
}

fn main() {
    let mut log = Vec::new();
    for t in 0..7 {
        println!("{}", bail(t, &mut log));
    }
}
