// An `Option` a test proved there, stored through a `RefCell`'s borrow of
// a nullable field, `*value = deferred` under `deferred.is_some()`, is
// stored as it is, MIR's answer, React's Flight client's `deferred`; one
// written again between, `None`, is stored `null`, as Rust's `None`, so the
// crate's `=== null` reads it so (#1933 step 4 (B)).
use std::cell::RefCell;

enum State {
    Blocked {
        #[cfg_attr(rust_js, rust_js::nullable)]
        value: Option<Vec<u32>>,
    },
    Done,
}

struct Chunk {
    state: RefCell<State>,
}

fn requeue_kept(chunk: &Chunk, items: &[u32]) {
    let mut deferred: Option<Vec<u32>> = None;
    for &item in items {
        if item > 1 {
            deferred.get_or_insert_with(Vec::new).push(item);
        }
    }
    if deferred.is_some()
        && let State::Blocked { value } = &mut *chunk.state.borrow_mut()
    {
        *value = deferred;
    }
}

fn requeue(chunk: &Chunk, items: &[u32], reset: bool) {
    let mut deferred: Option<Vec<u32>> = None;
    for &item in items {
        if item > 1 {
            deferred.get_or_insert_with(Vec::new).push(item);
        }
    }
    if deferred.is_some()
        && let State::Blocked { value } = &mut *chunk.state.borrow_mut()
    {
        if reset {
            deferred = None;
        }
        *value = deferred;
    }
}

fn shown(chunk: &Chunk) -> String {
    match &*chunk.state.borrow() {
        State::Blocked { value } => format!("{} {:?}", value.is_some(), value),
        State::Done => "done".to_string(),
    }
}

fn main() {
    let chunk = Chunk {
        state: RefCell::new(State::Blocked { value: Some(vec![0]) }),
    };
    requeue_kept(&chunk, &[1, 2, 3]);
    println!("{}", shown(&chunk));
    requeue(&chunk, &[1, 6], false);
    println!("{}", shown(&chunk));
    requeue(&chunk, &[5], true);
    println!("{}", shown(&chunk));
    requeue_kept(&chunk, &[1]);
    println!("{}", shown(&chunk));
    let done = Chunk {
        state: RefCell::new(State::Done),
    };
    requeue(&done, &[4], false);
    println!("{}", shown(&done));
}
