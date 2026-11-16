// An integer a closure captures `&mut`, the closure given only to a call as
// a `&mut dyn FnMut`, React's Flight loaders' `next_entry_idx` given
// `read_mappings` (#1947): its value is forgotten past the call, not lent
// away for good, so a fresh test of it past the call bounds an index, and
// `seen[next]` reads no check. A raw pointer the closure, or what it calls,
// could keep of it is refused (ADR 1114): the integer is a boxed copy.
// Given `&mut` to a call itself, `bump(&mut i)`, it's forgotten past the
// call too; but where the call may keep a pointer to it, `Keeper::keep`,
// or generic code's `K::keep`, which can't be followed, it's lent away from
// then on, so a test before a call that writes through that pointer bounds
// nothing, and `v[i]` panics as Rust's.
use std::panic;

fn read_all(text: &str, each: &mut dyn FnMut(usize, char)) {
    for (at, c) in text.char_indices() {
        each(at, c);
    }
}

fn marked(text: &str, marks: &[usize]) -> Vec<u32> {
    let mut seen = vec![0u32; marks.len()];
    let mut next = 0usize;
    let mut last = 0usize;
    read_all(text, &mut |at, _| {
        if next < marks.len() && marks[next] == at {
            next += 1;
        }
        last = at;
    });
    if next < seen.len() && last > 0 {
        seen[next] += 1;
    }
    seen
}

/// The closure called again by a second call: what the first left is
/// forgotten past the second too.
fn twice(text: &str, marks: &[usize]) -> Vec<u32> {
    let mut seen = vec![0u32; marks.len() + 1];
    let mut next = 0usize;
    let mut step = |at: usize, _: char| {
        if next < marks.len() && marks[next] == at {
            next += 1;
        }
    };
    read_all(text, &mut step);
    read_all(text, &mut step);
    if next < seen.len() {
        seen[next] += 1;
    }
    seen
}

fn bump(n: &mut usize) {
    *n += 1;
}

fn bumped(v: &[u32]) -> u32 {
    let mut i = 0usize;
    bump(&mut i);
    bump(&mut i);
    if i < v.len() { v[i] } else { 0 }
}

struct Keeper {
    at: *mut usize,
}

trait Keep {
    fn keep(&mut self, n: &mut usize);
    fn write(&self, value: usize);
}

impl Keep for Keeper {
    fn keep(&mut self, n: &mut usize) {
        self.at = n;
    }

    fn write(&self, value: usize) {
        unsafe { *self.at = value };
    }
}

fn kept(v: &[u32]) -> u32 {
    let mut i = 0usize;
    let mut keeper = Keeper {
        at: std::ptr::null_mut(),
    };
    keeper.keep(&mut i);
    if i < v.len() {
        keeper.write(9);
        v[i]
    } else {
        0
    }
}

fn kept_by<K: Keep>(keeper: &mut K, v: &[u32]) -> u32 {
    let mut i = 0usize;
    keeper.keep(&mut i);
    if i < v.len() {
        keeper.write(9);
        v[i]
    } else {
        0
    }
}

fn main() {
    println!("{:?}", marked("abcdef", &[1, 3, 9]));
    println!("{:?}", marked("ab", &[5]));
    println!("{:?}", twice("abcd", &[0, 2, 1]));
    println!("bumped {}", bumped(&[1, 2, 3]));
    match panic::catch_unwind(|| kept(&[1, 2, 3])) {
        Ok(value) => println!("kept {value}"),
        Err(_) => println!("kept panicked"),
    }
    let mut keeper = Keeper {
        at: std::ptr::null_mut(),
    };
    match panic::catch_unwind(panic::AssertUnwindSafe(|| kept_by(&mut keeper, &[1, 2, 3]))) {
        Ok(value) => println!("kept by generic code {value}"),
        Err(_) => println!("kept by generic code panicked"),
    }
}
