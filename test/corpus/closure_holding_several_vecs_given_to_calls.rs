// A closure that captures several `Vec`s `&mut`, given to a call, holds
// each of them: the call forgets every one's length, not only the last
// captured's, so `a[2]` past a call whose closure cleared `a` is checked and
// panics as Rust's does, whether `a` is captured first or last, of two or
// three, given as a `&mut dyn FnMut` or called itself (#1977).
use std::panic;

fn each(f: &mut dyn FnMut()) {
    f();
}

fn two_cleared_first() -> u32 {
    let mut a: Vec<u32> = vec![1, 2, 3];
    let mut b: Vec<u32> = vec![4, 5, 6];
    let n = a.len();
    each(&mut || {
        a.clear();
        b.push(7);
    });
    if n > 2 { a[2] } else { 0 }
}

fn two_cleared_last() -> u32 {
    let mut a: Vec<u32> = vec![1, 2, 3];
    let mut b: Vec<u32> = vec![4, 5, 6];
    let n = a.len();
    each(&mut || {
        b.push(7);
        a.clear();
    });
    if n > 2 { a[2] } else { 0 }
}

fn three_cleared_first() -> u32 {
    let mut a: Vec<u32> = vec![1, 2, 3];
    let mut b: Vec<u32> = vec![4, 5, 6];
    let mut c: Vec<u32> = vec![7, 8, 9];
    let n = a.len();
    each(&mut || {
        a.clear();
        b.push(7);
        c.pop();
    });
    if n > 2 { a[2] } else { 0 }
}

fn three_cleared_last() -> u32 {
    let mut a: Vec<u32> = vec![1, 2, 3];
    let mut b: Vec<u32> = vec![4, 5, 6];
    let mut c: Vec<u32> = vec![7, 8, 9];
    let n = a.len();
    each(&mut || {
        b.push(7);
        c.pop();
        a.clear();
    });
    if n > 2 { a[2] } else { 0 }
}

fn called_itself() -> u32 {
    let mut a: Vec<u32> = vec![1, 2, 3];
    let mut b: Vec<u32> = vec![4, 5, 6];
    let n = a.len();
    let mut step = || {
        a.clear();
        b.push(7);
    };
    step();
    if n > 2 { a[2] } else { 0 }
}

fn grown() -> u32 {
    let mut a: Vec<u32> = vec![1, 2, 3];
    let mut b: Vec<u32> = vec![4, 5, 6];
    let n = a.len();
    each(&mut || {
        b.clear();
        a.push(4);
    });
    if n > 2 { a[3] + b.len() as u32 } else { 0 }
}

fn run(name: &str, row: fn() -> u32) {
    match panic::catch_unwind(row) {
        Ok(value) => println!("{name}: {value}"),
        Err(_) => println!("{name}: panicked"),
    }
}

fn main() {
    run("two, cleared first", two_cleared_first);
    run("two, cleared last", two_cleared_last);
    run("three, cleared first", three_cleared_first);
    run("three, cleared last", three_cleared_last);
    run("called itself", called_itself);
    run("grown", grown);
}
