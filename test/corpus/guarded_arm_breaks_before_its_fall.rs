// A guarded arm that breaks its loop, `1 if on(..) => break`, before the
// arm its value falls to, `1 | 2 => { .. }`, which leaves the function in
// a `let .. else`-like `if`, and an arm whose `if` chain breaks the loop
// too: the `if` chain Rust tries them as, no label (#389c). MIR leaves the
// first arm by a block the second is placed after.
use std::cell::RefCell;

fn on(log: &RefCell<Vec<String>>, n: u32) -> bool {
    log.borrow_mut().push(format!("on:{n}"));
    n % 3 == 0
}

fn tally(tags: &[u32], log: &RefCell<Vec<String>>) -> u32 {
    let mut n = 0;
    for &t in tags {
        match t {
            1 if on(log, n) => break,
            1 | 2 => {
                let m = if n > 4 { 1 } else { return 0 };
                n += m;
            }
            3 => {
                if n > 2 {
                    n += 4
                } else if n == 1 {
                    break;
                } else {
                    n += 9
                }
            }
            _ => {}
        }
        n += 1;
    }
    n
}

fn main() {
    let log = RefCell::new(Vec::new());
    for tags in [
        &[][..],
        &[1],
        &[3, 1],
        &[3, 2, 2],
        &[3, 3, 1, 1],
        &[4, 3, 3, 2],
        &[0, 3, 5, 3],
        &[3, 3, 1, 2, 1, 4],
        &[2],
    ] {
        let n = tally(tags, &log);
        println!("{tags:?}: {n} {:?}", log.take());
    }
}
