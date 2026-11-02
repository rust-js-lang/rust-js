// An enum one `match` makes in each arm and the next takes apart at once
// (rd433): React's Flight client's `then`, which reads what's settled
// while the chunk's state is borrowed and calls back once it isn't, as a
// callback may read the chunk again. Each arm goes on to its variant's
// case, one dispatch, `if (..) { resolve(value) }`, as upstream's one
// `switch` calls back, never an object of a tag tested again. A variant
// two arms make, one of nothing, a case `_` takes, one in a loop, one of
// two fields and an or-pattern of two variants; and where something runs
// between, the enum is read whole, its payload's parts are bound apart, or
// it holds what drops, it's made.
use std::cell::RefCell;

pub enum State {
    Pending(Vec<String>),
    Done(i32),
    Failed(&'static str),
    Lost(&'static str),
    Halted,
}

pub struct Chunk {
    state: RefCell<State>,
}

fn then(chunk: &Chunk, resolve: Option<&dyn Fn(i32)>, reject: Option<&dyn Fn(&str)>) {
    let settled = match &mut *chunk.state.borrow_mut() {
        State::Done(value) => Ok(*value),
        State::Pending(listeners) => {
            listeners.push("listener".to_string());
            return;
        }
        State::Halted => return,
        State::Failed(reason) => Err(*reason),
        State::Lost(reason) => Err(*reason),
    };
    // A callback may read the chunk again, so it's called once the state
    // is no longer borrowed.
    match settled {
        Ok(value) => {
            if let Some(resolve) = resolve {
                resolve(value);
            }
        }
        Err(reason) => {
            if let Some(reject) = reject {
                reject(reason);
            }
        }
    }
}

fn describe(chunk: &Chunk) -> String {
    let done = match &*chunk.state.borrow() {
        State::Done(value) => Some(*value),
        _ => None,
    };
    if let Some(value) = done {
        return format!("done {value}");
    }
    "not done".to_string()
}

fn first_reason(chunks: &[Chunk]) -> Option<&'static str> {
    for chunk in chunks {
        let reason = match &*chunk.state.borrow() {
            State::Failed(reason) | State::Lost(reason) => Some(*reason),
            _ => None,
        };
        match reason {
            Some(reason) => return Some(reason),
            _ => continue,
        }
    }
    None
}

fn shown(chunk: &Chunk) -> i32 {
    let settled = match &*chunk.state.borrow() {
        State::Done(value) => Ok(*value),
        _ => Err(()),
    };
    println!("between");
    match settled {
        Ok(value) => value,
        Err(()) => -1,
    }
}

fn counted(chunk: &Chunk, count: i32) -> i32 {
    let settled = match &*chunk.state.borrow() {
        State::Done(value) => Some(*value),
        _ => None,
    };
    let small = count < 10;
    match settled {
        Some(value) => {
            if small {
                value
            } else {
                count
            }
        }
        None => -count,
    }
}

fn kept(chunk: &Chunk) -> Result<i32, &'static str> {
    let settled = match &*chunk.state.borrow() {
        State::Done(value) => Ok(*value),
        _ => Err("unsettled"),
    };
    match settled {
        Ok(value) => Ok(value * 2),
        Err(_) => settled,
    }
}

pub enum Pair {
    Both(i32, i32),
    One(i32),
}

fn sum(n: i32) -> i32 {
    let pair = match n {
        0 => Pair::One(7),
        _ => Pair::Both(n, n * 2),
    };
    match pair {
        Pair::Both(a, b) => a + b,
        Pair::One(a) => a,
    }
}

fn either(n: i32) -> i32 {
    let side = if n > 0 { Ok(n) } else { Err(-n) };
    match side {
        Ok(v) | Err(v) => v * 3,
    }
}

fn parts(chunks: &[Chunk]) -> String {
    let first = match chunks.first() {
        Some(chunk) => Some((describe(chunk), chunks.len())),
        None => None,
    };
    if let Some((text, count)) = first {
        return format!("{text} of {count}");
    }
    String::new()
}

// A tagged enum's payload, a tuple bound apart, is threaded as its parts,
// each read of its own, never the tuple's `const [text, count]`.
fn tagged_parts(chunks: &[Chunk], skipped: usize) -> String {
    let first = match chunks.first() {
        Some(chunk) => Ok((describe(chunk), chunks.len())),
        None => Err(skipped),
    };
    match first {
        Ok((text, count)) => format!("{text} of {count}"),
        Err(skipped) => format!("none, {skipped} skipped"),
    }
}

enum Settled {
    Done(i32),
    Failed(&'static str),
}

// The second `match` is reached from where the first made nothing too: a
// value set before it, kept where `fresh` is false, is no dispatch.
fn entered(fresh: bool, n: i32) -> i32 {
    let mut settled = Settled::Failed("stale");
    if fresh {
        settled = match n {
            0 => Settled::Failed("zero"),
            _ => Settled::Done(n * 2),
        };
    }
    match settled {
        Settled::Done(v) => v,
        Settled::Failed(why) => why.len() as i32,
    }
}

// The arm's variable is written again after it's bound, so it isn't the
// field's own.
fn rebound(n: i32) -> i32 {
    let settled = if n > 0 { Ok(n) } else { Err(-n) };
    match settled {
        Ok(v) => v,
        Err(mut reason) => {
            reason += 100;
            reason
        }
    }
}

// The same, made in two places: what each makes is a value of its own,
// which the arm's variable is bound to, as the arm writes it again, not a
// write of that variable (#479: since a00a916cd, `rebound` alone is the
// same JS either way).
fn rebound_made_twice(a: bool, b: bool) -> i32 {
    let settled = if a {
        Err(1)
    } else if b {
        Err(2)
    } else {
        Ok(3)
    };
    match settled {
        Ok(v) => v,
        Err(mut reason) => {
            reason *= 2;
            reason
        }
    }
}

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

fn loud(n: i32) {
    let made = match n {
        0 => Ok(Loud("zero")),
        _ => Err(n),
    };
    match made {
        Ok(_) => println!("made"),
        Err(n) => println!("not made {n}"),
    }
    println!("after");
}

// `Ok` and `Err` each made twice, whose cases go on to what follows:
// left whole, as each case can't be in its arms without a label.
fn scaled(state: &State, n: i32) -> i32 {
    let made = match state {
        State::Done(_) => Ok(n),
        State::Failed(_) => Err(n + 1),
        State::Pending(_) => Ok(n * 2),
        State::Lost(_) | State::Halted => Err(7),
    };
    let scaled = match made {
        Ok(v) => v * 10,
        Err(e) => {
            println!("err {e}");
            e
        }
    };
    println!("scaled");
    scaled
}

// An `Option` an `if` chain makes, its value or `undefined`, taken by the
// `let`-`else` after it, with no borrow between: the value the program
// named, tested after, as ReactART's createInstance's `instance` and `if
// (!instance) throw` (rd531), not threaded.
fn instance_of(kind: &str) -> i32 {
    let instance = if kind == "clip" {
        let instance = kind.len() as i32;
        println!("clipping {instance}");
        Some(instance)
    } else if kind == "group" {
        Some(2)
    } else {
        None
    };
    let Some(instance) = instance else {
        return -1;
    };
    instance * 10
}

fn main() {
    println!("{} {} {}", instance_of("clip"), instance_of("group"), instance_of("text"));
    let chunk = Chunk {
        state: RefCell::new(State::Done(4)),
    };
    let reads = |v: i32| {
        // The chunk is no longer borrowed.
        println!("resolved {v}, {}", describe(&chunk));
    };
    let rejects = |r: &str| println!("rejected {r}, {}", describe(&chunk));
    then(&chunk, Some(&reads), Some(&rejects));
    *chunk.state.borrow_mut() = State::Failed("no");
    then(&chunk, Some(&reads), Some(&rejects));
    *chunk.state.borrow_mut() = State::Lost("gone");
    then(&chunk, Some(&reads), Some(&rejects));
    then(&chunk, Some(&reads), None);
    *chunk.state.borrow_mut() = State::Pending(vec![]);
    then(&chunk, Some(&reads), None);
    if let State::Pending(listeners) = &*chunk.state.borrow() {
        println!("{} listening", listeners.len());
    }
    *chunk.state.borrow_mut() = State::Halted;
    then(&chunk, None, Some(&rejects));
    println!("{}", describe(&chunk));
    let chunks = [
        Chunk {
            state: RefCell::new(State::Done(1)),
        },
        Chunk {
            state: RefCell::new(State::Lost("lost")),
        },
        Chunk {
            state: RefCell::new(State::Failed("failed")),
        },
    ];
    println!("{:?}", first_reason(&chunks));
    println!("{:?}", first_reason(&chunks[..1]));
    println!("{} {}", shown(&chunks[0]), shown(&chunks[1]));
    println!("{} {}", counted(&chunks[0], 2), counted(&chunks[1], 2));
    println!("{:?} {:?}", kept(&chunks[0]), kept(&chunks[2]));
    println!("{} {} {} {}", sum(0), sum(4), either(5), either(-6));
    println!("{:?} {:?}", parts(&chunks), parts(&[]));
    println!("{:?} {:?}", tagged_parts(&chunks, 1), tagged_parts(&[], 2));
    println!("{} {} {}", entered(true, 0), entered(true, 4), entered(false, 9));
    println!("{} {}", rebound(3), rebound(-3));
    println!(
        "{} {} {}",
        rebound_made_twice(true, false),
        rebound_made_twice(false, true),
        rebound_made_twice(false, false)
    );
    println!("{}", scaled(&State::Pending(vec![]), 2) + scaled(&State::Halted, 2));
    loud(0);
    loud(3);
}
