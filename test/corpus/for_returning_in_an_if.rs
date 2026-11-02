// A `for` in an `if` whose branch returns after it, its body returning
// early too, as React's Flight client's getOutlinedModel walks a path:
// `for (const step of path)`, not a `while` of the iterator's `next()`
// (rd602), and of a range, `for (let i = 1; i < len; i++)`, not
// `$rangeNext` (rd603); a panic in it says what Rust's does, and what's
// held is dropped where Rust drops it.
use std::panic;

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn is_lazy(value: u32) -> bool {
    value > 100
}

fn wait(step: u32) -> u32 {
    step + 1000
}

fn walk(ok: bool, path: &[u32], mut value: u32) -> u32 {
    if ok {
        for step in path {
            if is_lazy(value) {
                return wait(*step);
            }
            value += step;
        }
        return value;
    }
    7
}

fn walk_owned(ok: bool, path: Vec<u32>, mut value: u32) -> u32 {
    if ok {
        for step in path {
            while is_lazy(value) {
                if value > 500 {
                    return wait(step);
                }
                value -= 100;
            }
            value += step;
        }
        return value;
    }
    7
}

fn walk_range(ok: bool, path: &[u32], mut value: u32) -> u32 {
    if ok {
        let len = path.len();
        for i in 1..len {
            while is_lazy(value) {
                if value > 500 {
                    return wait(path[i - 1]);
                }
                value -= 100;
            }
            let name = path[i];
            if name < 50 {
                value += name;
            } else {
                panic!("Invalid reference.");
            }
        }
        return value;
    }
    7
}

fn main() {
    for (ok, path, value) in [
        (true, vec![1, 2, 3], 0),
        (true, vec![1, 60, 3], 99),
        (true, vec![], 5),
        (false, vec![1], 0),
    ] {
        println!("walk {}", walk(ok, &path, value));
        println!("owned {}", walk_owned(ok, path.clone(), value));
    }
    println!("owned {}", walk_owned(true, vec![450, 2, 3], 90));
    for (path, value) in [(vec![0, 1, 2], 0), (vec![5, 6, 7], 650), (vec![5, 6, 7], 150)] {
        println!("range {}", walk_range(true, &path, value));
    }
    let thrown = panic::catch_unwind(|| {
        let _held = Loud("held");
        walk_range(true, &[1, 2, 70], 0)
    });
    println!("thrown {}", thrown.is_err());
    println!("not ok {}", walk_range(false, &[], 0));
}
