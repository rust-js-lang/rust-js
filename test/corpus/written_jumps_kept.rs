// Each `continue` and loop `break` the program wrote is kept where it's
// written, though the turn or the function ends there anyway (the user's
// decision, 2026-10-29; ADR 0684, amended): React DOM's
// replayUnblockedFormActions, `if (blockedOn === null) { ..; continue; }
// else { break; }`, the loop the function ends with, whose `break` MIR has
// as its `return;`, and what follows an `if` written without an `else`
// after it; a written `else` kept beside its `then`'s `continue`; an
// `else`'s `continue` at the turn's end; and an `else`'s `break` of the
// loop a function ends with; and `if c { continue; } rest` inside an arm,
// which MIR turns `if (!c) { rest }`, as written, React's findAllNodes'
// `if (isHiddenSubtree(node)) { continue; }`, of a rest that shadows a
// name the arm declares before it too. Where a value's drop follows the `if`,
// the `continue` stays MIR's: the turn doesn't end after the branch. A
// loop a branch of the function's last `if` ends with, React DOM's
// warnForBlockInsideInline's in its `if (__DEV__)`, keeps its `break`s too.
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

fn roots(tree: &[(u8, bool, i32)], out: &mut Vec<i32>) {
    let mut index = 0;
    while index < tree.len() {
        let (tag, hidden, value) = tree[index];
        index += 1;
        if tag == 5 || tag == 6 {
            if hidden {
                continue;
            }
            out.push(value);
        } else {
            out.push(-value);
        }
    }
}

fn shadowed(tree: &[(u8, bool, i32)], out: &mut Vec<i32>) {
    for &(tag, hidden, value) in tree {
        if tag == 5 {
            let doubled = value * 2;
            out.push(doubled);
            if hidden {
                continue;
            }
            let doubled = doubled + 1;
            out.push(doubled);
        } else {
            out.push(0);
        }
    }
}

fn last_continue(xs: &[i32], out: &mut Vec<i32>) {
    for &x in xs {
        out.push(0);
        if x > 3 {
            out.push(x);
            continue;
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

fn in_dev(xs: &[i32], dev: bool, out: &mut Vec<i32>) {
    if dev {
        let mut next = xs.first().copied();
        while let Some(x) = next {
            if x > 3 {
                out.push(x);
                break;
            }
            if x < 0 {
                break;
            }
            next = xs.get(x as usize).copied();
        }
    } else {
        for &x in xs {
            if x == 0 {
                break;
            }
            out.push(-x);
        }
    }
}

// A `match`'s `None => break` in the loop a function ends with, React
// DOM's hideOrUnhideDehydratedBoundary: a `break`, not MIR's `return;`.
fn unhide(next: &[usize], start: usize, out: &mut Vec<usize>) {
    let mut node = start;
    let mut depth = 0;
    loop {
        let next_node = next.get(node).copied().filter(|&n| n != 0);
        out.push(node);
        if node > 4 {
            if depth == 0 {
                return;
            } else {
                depth -= 1;
            }
        } else if node == 2 {
            depth += 1;
        }
        match next_node {
            Some(next_node) => node = next_node,
            None => break,
        }
    }
}

// And where the arm does more first, or another arm returns.
fn arm_ends(next: &[usize], start: usize, out: &mut Vec<usize>) {
    let mut node = start;
    loop {
        out.push(node);
        match next.get(node).copied() {
            Some(0) => return,
            Some(next_node) => node = next_node,
            None => {
                out.push(99);
                break;
            }
        }
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
    let tree = [(5, false, 1), (6, true, 2), (1, false, 3), (6, false, 4)];
    roots(&tree, &mut out);
    shadowed(&tree, &mut out);
    shadowed(&[(5, true, 7)], &mut out);
    last_continue(&[2, 8], &mut out);
    println!("{out:?}");
    dropped_after(&[1, 5]);
    let mut dev = Vec::new();
    in_dev(&[1, 2, 5], true, &mut dev);
    in_dev(&[2, 0, -1], true, &mut dev);
    in_dev(&[3, 4, 0, 6], false, &mut dev);
    println!("{dev:?}");
    let mut walked = Vec::new();
    unhide(&[3, 0, 6, 2, 9, 1, 7, 0, 0, 0], 0, &mut walked);
    unhide(&[1, 2, 5, 0, 0, 7, 0, 8, 0], 0, &mut walked);
    unhide(&[1, 3], 0, &mut walked);
    arm_ends(&[2, 0, 1], 0, &mut walked);
    arm_ends(&[2, 0, 5], 0, &mut walked);
    println!("{walked:?}");
}
