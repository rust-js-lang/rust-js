// A comment above a last `_ => {}`, after a guarded arm that falls to the
// next guarded arm of the same value, changes neither the `switch` nor
// where what follows the `match` is: each case's `break;`, one tail after
// the `switch`, no label, and the comment in the last case, as React's
// attemptEarlyBailoutIfNoScheduledUpdate writes its `// Fallthrough`
// (rd702). Of a tail that's a call, a value, and a sum.
const ENABLE_A: bool = true;
const ENABLE_B: bool = false;
const ENABLE_C: bool = true;

fn push(log: &mut Vec<u32>, n: u32) {
    log.push(n);
}

fn finish(log: &mut Vec<u32>) -> Option<u32> {
    Some(log.len() as u32)
}

pub fn bail(tag: u32, log: &mut Vec<u32>) -> Option<u32> {
    match tag {
        0 => {
            push(log, 0);
        }
        1 => {
            push(log, 1);
        }
        4 if ENABLE_A => {
            push(log, 4);
        }
        4 | 5 if ENABLE_B => {
            push(log, 9);
            return log.pop();
        }
        // Fallthrough
        _ => {}
    }
    finish(log)
}

pub fn kept(tag: u32, lanes: &mut u32) -> u32 {
    match tag {
        3 => {
            *lanes += 3;
        }
        25 if ENABLE_B => {
            *lanes += 25;
        }
        // Fallthrough
        25 | 23 if ENABLE_C => {
            *lanes = 0;
            return 7;
        }
        // Fallthrough
        _ => {}
    }
    tag
}

pub fn summed(tag: u32, lanes: &mut u32) -> u32 {
    match tag {
        3 => {
            *lanes += 3;
        }
        5 => {
            *lanes += 5;
        }
        25 if ENABLE_B => {
            *lanes += 25;
        }
        25 | 23 if ENABLE_C => {
            *lanes = 0;
            return 7;
        }
        // Fallthrough
        _ => {}
    }
    tag + 1
}

fn main() {
    let mut log = Vec::new();
    for t in 0..7 {
        println!("{:?}", bail(t, &mut log));
    }
    let mut lanes = 1;
    for t in [3, 5, 23, 25, 9] {
        let k = kept(t, &mut lanes);
        let s = summed(t, &mut lanes);
        println!("{t}: {k} {s} {lanes}");
    }
}
