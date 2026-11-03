// A comment above a last `_ => {}`, after guarded arms that fall, in a
// `match` with an arm that returns, is in the last case above its
// `break;`, and the tail is after the `switch` once: not `return
// finish(log);` in each case, the comment in the first, as a `return`
// arm left a `break` of MIR's block after the comment's mark (rd702). As
// React's attemptEarlyBailoutIfNoScheduledUpdate, whose ACTIVITY and
// SUSPENSE arms return, writes its last `// Fallthrough`.
const ENABLE_A: bool = true;
const ENABLE_B: bool = false;

fn push(log: &mut Vec<u32>, n: u32) {
    log.push(n);
}

fn finish(log: &mut Vec<u32>) -> Option<u32> {
    Some(log.len() as u32)
}

fn bail(tag: u32, log: &mut Vec<u32>, instance: Option<u32>) -> Option<u32> {
    // This fiber does not have any pending work.
    match tag {
        0 => {
            push(log, 0);
        }
        1 => {
            push(log, 1);
            return None;
        }
        4 if ENABLE_A => {
            if let Some(instance) = instance {
                push(log, instance);
            }
        }
        // Fallthrough
        4 | 5 if ENABLE_B => {
            push(log, 9);
            return log.pop();
        }
        // Fallthrough
        _ => {}
    }
    finish(log)
}

fn main() {
    let mut log = Vec::new();
    for t in 0..7 {
        println!("{:?} {:?}", bail(t, &mut log, Some(t * 10)), bail(t, &mut log, None));
    }
    println!("{log:?}");
}
