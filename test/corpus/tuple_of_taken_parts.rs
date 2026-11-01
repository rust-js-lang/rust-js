// A tuple an `if`'s or a `match`'s branches each make of parts taken
// first, `(value.take(), reason.take())`, then taken apart, `let (a, b) =
// ..`: each part a variable of its own, assigned in each branch, never an
// array made and read item by item (rd524, React's Flight client's
// resolveModelChunk).
use std::cell::RefCell;

enum State {
    Pending {
        value: Option<Vec<u32>>,
        reason: Option<Vec<u32>>,
    },
    Blocked {
        value: Option<Vec<u32>>,
        reason: Option<Vec<u32>>,
    },
    Resolved(u32),
}

struct Chunk {
    state: RefCell<State>,
}

fn resolve(chunk: &Chunk, resolved: u32) -> (usize, usize) {
    let (resolve_listeners, reject_listeners) = match &mut *chunk.state.borrow_mut() {
        State::Pending { value, reason } | State::Blocked { value, reason } => (value.take(), reason.take()),
        State::Resolved(_) => (None, None),
    };
    *chunk.state.borrow_mut() = State::Resolved(resolved);
    (
        resolve_listeners.map_or(0, |listeners| listeners.len()),
        reject_listeners.map_or(0, |listeners| listeners.len()),
    )
}

fn taken(c: bool, x: &mut Option<Vec<u32>>, y: &mut Option<Vec<u32>>) -> usize {
    let (a, b) = if c { (x.take(), y.take()) } else { (None, None) };
    a.map_or(0, |a| a.len()) * 10 + b.map_or(0, |b| b.len())
}

// A branch that returns makes none: it's the guard before the tuple.
fn early(n: i32, x: &mut Option<Vec<u32>>) -> usize {
    let (a, b) = if n > 0 {
        (x.take(), n)
    } else if n == 0 {
        return 7;
    } else {
        (None, -n)
    };
    a.map_or(0, |a| a.len()) + b as usize
}

// A branch that panics makes none either.
fn checked(n: i32, x: &mut Option<Vec<u32>>) -> usize {
    let (a, b) = if n > 0 {
        (x.take(), n)
    } else if n == 0 {
        (None, 1)
    } else {
        panic!("negative {n}");
    };
    a.map_or(0, |a| a.len()) + b as usize
}

// A part held once and given twice stays held.
fn twice(c: bool, x: &mut Option<Vec<u32>>) -> usize {
    let (a, b) = if c {
        let n = x.take().map_or(0, |v| v.len());
        (n, n)
    } else {
        (1, 2)
    };
    a * 10 + b
}

fn main() {
    let chunk = Chunk {
        state: RefCell::new(State::Pending {
            value: Some(vec![1, 2]),
            reason: Some(vec![3]),
        }),
    };
    println!("{:?}", resolve(&chunk, 4));
    println!("{:?}", resolve(&chunk, 5));
    let blocked = Chunk {
        state: RefCell::new(State::Blocked {
            value: None,
            reason: Some(vec![1, 2, 3]),
        }),
    };
    println!("{:?}", resolve(&blocked, 6));
    if let State::Resolved(n) = *blocked.state.borrow() {
        println!("resolved {n}");
    }
    let mut x = Some(vec![1, 2, 3]);
    let mut y = Some(vec![4]);
    println!("{}", taken(false, &mut x, &mut y));
    println!("{}", taken(true, &mut x, &mut y));
    println!("{:?} {:?}", x, y);
    let mut z = Some(vec![9, 9]);
    println!("{} {} {}", early(0, &mut z), early(-3, &mut z), early(2, &mut z));
    println!("{:?}", z);
    let mut v = Some(vec![1]);
    println!("{} {}", checked(0, &mut v), checked(2, &mut v));
    let mut w = Some(vec![5, 6, 7]);
    println!("{} {} {:?}", twice(true, &mut w), twice(false, &mut w), w);
}
