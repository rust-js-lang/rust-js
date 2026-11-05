// A flattened state's `RefCell::replace` whose new value calls first, so
// MIR holds the cell in a variable before the call: what it gives back is
// a copy of its variant's keys, read after the new value's are set, never
// the object those sets change (#786, ADR 1044).
use std::cell::RefCell;
use std::rc::Rc;

#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum ChunkState {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending { value: Option<u32> },
    #[cfg_attr(rust_js, rust_js::name = "fulfilled")]
    Fulfilled { value: u32, reason: Option<u32> },
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct ReactPromise {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub state: RefCell<ChunkState>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl ReactPromise {
    fn new(state: ChunkState) -> Self {
        ReactPromise {
            state: RefCell::new(state),
        }
    }
}

type Chunk = Rc<ReactPromise>;

fn next_value(n: u32) -> u32 {
    print!("next ");
    n + 1
}

fn fulfilled_was(chunk: &Chunk, n: u32) -> String {
    let old = chunk.state.replace(ChunkState::Fulfilled {
        value: next_value(n),
        reason: Some(n),
    });
    match old {
        ChunkState::Pending { value } => format!("was pending {value:?}"),
        ChunkState::Fulfilled { value, reason } => format!("was fulfilled {value} {reason:?}"),
    }
}

fn reset_was_fulfilled(chunk: &Chunk, n: u32) -> bool {
    let old = chunk.state.replace(ChunkState::Pending {
        value: Some(next_value(n)),
    });
    matches!(old, ChunkState::Fulfilled { .. })
}

fn describe(chunk: &Chunk) -> String {
    match &*chunk.state.borrow() {
        ChunkState::Pending { value } => format!("pending {value:?}"),
        ChunkState::Fulfilled { value, reason } => format!("fulfilled {value} {reason:?}"),
    }
}

fn main() {
    let chunk = Rc::new(ReactPromise::new(ChunkState::Pending { value: Some(1) }));
    println!("{} | {}", fulfilled_was(&chunk, 3), describe(&chunk));
    println!("{} | {}", fulfilled_was(&chunk, 5), describe(&chunk));
    println!("{} | {}", reset_was_fulfilled(&chunk, 7), describe(&chunk));
    println!("{} | {}", reset_was_fulfilled(&chunk, 9), describe(&chunk));
}
