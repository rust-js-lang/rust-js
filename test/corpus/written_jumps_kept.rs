// Each `continue` and loop `break` the program wrote is kept where it's
// written, though the turn or the function ends there anyway (the user's
// decision, 2026-10-29; ADR 0684, amended): React DOM's
// replayUnblockedFormActions, `if (blockedOn === null) { ..; continue; }
// else { break; }`, the loop the function ends with, whose `break` MIR has
// as its `return;`, and what follows an `if` written without an `else`
// after it; a written `else` kept beside its `then`'s `continue`; an
// `else`'s `continue` at the turn's end; and an `else`'s `break` of the
// loop a function ends with. Where a value's drop follows the `if`, the
// `continue` stays MIR's: the turn doesn't end after the branch.
struct Noisy(i32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn blocked(x: i32) -> Option<i32> {
    (x > 10).then_some(x)
}

fn ours(x: i32) -> Option<i32> {
    (x > 5).then_some(x)
}

fn replay(queue: &mut Vec<i32>, log: &mut Vec<i32>) {
    let mut i = 0;
    while i < queue.len() {
        let action = queue[i];
        if action < 0 {
            if blocked(-action).is_none() {
                i += 3;
                continue;
            } else {
                break;
            }
        }
        if let Some(inst) = ours(action) {
            queue.remove(i);
            log.push(inst);
            continue;
        }
        i += 3;
    }
}

fn kept_else(xs: &[i32], out: &mut Vec<i32>) {
    for &x in xs {
        if x > 3 {
            out.push(x);
            continue;
        } else {
            out.push(0);
        }
    }
}

fn else_continues(xs: &[i32], out: &mut Vec<i32>) {
    let mut i = 0;
    loop {
        if i >= xs.len() {
            break;
        }
        let x = xs[i];
        i += 1;
        if x > 0 {
            out.push(x);
        } else {
            out.push(-x);
            continue;
        }
    }
}

fn else_breaks(xs: &[i32], out: &mut Vec<i32>) {
    for &x in xs {
        if x > 3 {
            out.push(x);
        } else {
            break;
        }
    }
}

fn dropped_after(xs: &[i32]) {
    for &x in xs {
        let n = Noisy(x);
        if x > 3 {
            println!("big {}", n.0);
            continue;
        }
        println!("small {}", n.0);
    }
}

fn main() {
    for queue in [vec![7, 1, 2, 3, -2, 0, 0, 8, -20, 9], vec![1, 0, 0, 6, -11, 5], vec![-1, 2, 3, 4]] {
        let (mut queue, mut log) = (queue, Vec::new());
        replay(&mut queue, &mut log);
        println!("{queue:?} {log:?}");
    }
    let mut out = Vec::new();
    kept_else(&[1, 5, 2, 9], &mut out);
    else_continues(&[3, -2, 0, 4], &mut out);
    else_breaks(&[5, 7, 1, 9], &mut out);
    println!("{out:?}");
    dropped_after(&[1, 5]);
}
