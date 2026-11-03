// #488: a guarded arm that only `continue`s, in a `match` that ends a loop's
// turn, goes past the `switch` either way, as its guard failing goes to
// `_ => {}`: `case 1: break;`, not `case 1: if (n > 3) { break; } break;`.
// A guard that does something is still run, `case 1: noted(log); break;`.

fn counted(v: &[u32], log: &mut Vec<u32>) -> u32 {
    let mut n = 0;
    for &x in v {
        match x {
            0 => {
                log.push(0);
                continue;
            }
            1 if n > 3 => continue,
            2 => n += 1,
            _ => {}
        }
    }
    n
}

fn noted(log: &mut Vec<u32>) -> bool {
    log.push(7);
    log.len() > 2
}

fn guarded(v: &[u32], log: &mut Vec<u32>) -> u32 {
    let mut n = 0;
    for &x in v {
        match x {
            1 if noted(log) => continue,
            2 => n += 1,
            _ => {}
        }
    }
    n
}

/// The last arm's, before `_ => {}`, which falls to nothing.
fn last_guarded(v: &[u32], log: &mut Vec<u32>) -> u32 {
    let mut n = 0;
    for &x in v {
        match x {
            2 => n += 1,
            4 => log.push(x),
            3 if n > 1 => continue,
            _ => {}
        }
    }
    n
}

fn main() {
    let mut log = Vec::new();
    println!("{}", last_guarded(&[3, 2, 4, 3, 2, 3, 1], &mut log));
    println!("{log:?}");
    let mut log = Vec::new();
    println!("{}", counted(&[0, 1, 2, 2, 1, 2, 2, 2, 1, 0], &mut log));
    println!("{log:?}");
    let mut log = Vec::new();
    println!("{}", guarded(&[1, 2, 1, 1, 2, 3], &mut log));
    println!("{log:?}");
}
