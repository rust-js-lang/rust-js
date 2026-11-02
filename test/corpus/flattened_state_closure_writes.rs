// A class's flattened state borrowed by a `let` of the guard's deref and
// written whole by a closure that captures the borrow, `let mut set = |v|
// *state = ChunkState::Initialized { .. }`, tested and taken from, or lent
// on, `|v| finish(state, v)`: each key written on the chunk itself, as a
// write in the function's own body is, never the variable the chunk is in,
// and the class's own fields kept (#482, ADR 1044).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum ChunkState {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending {
        value: Option<Vec<u32>>,
        reason: Option<String>,
    },
    #[cfg_attr(rust_js, rust_js::name = "blocked")]
    Blocked { value: Option<Vec<u32>> },
    #[cfg_attr(rust_js, rust_js::name = "fulfilled")]
    Initialized { value: String },
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct ReactPromise {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub state: RefCell<ChunkState>,
    pub reads: Cell<u32>,
    pub name: String,
}

#[cfg_attr(rust_js, rust_js::class)]
impl ReactPromise {
    fn new(state: ChunkState, name: &str) -> Self {
        ReactPromise {
            state: RefCell::new(state),
            reads: Cell::new(0),
            name: name.to_string(),
        }
    }
}

type Chunk = Rc<ReactPromise>;

fn written_by_a_closure(chunk: &Chunk) {
    let state = &mut *chunk.state.borrow_mut();
    let mut set = |v: &str| {
        *state = ChunkState::Initialized {
            value: v.to_string(),
        }
    };
    set("a");
    set("b");
}

fn blocked_by_a_closure(chunk: &Chunk) -> bool {
    let state = &mut *chunk.state.borrow_mut();
    let mut block = || {
        if let ChunkState::Pending { value, .. } = state {
            let listeners = value.take();
            *state = ChunkState::Blocked { value: listeners };
            return true;
        }
        false
    };
    block()
}

fn finish(state: &mut ChunkState, value: &str) {
    *state = ChunkState::Initialized { value: value.to_string() };
}

fn lent_by_a_closure(chunk: &Chunk) {
    let state = &mut *chunk.state.borrow_mut();
    let mut done = |v: &str| finish(state, v);
    done("c");
}

fn status(chunk: &Chunk) -> String {
    chunk.reads.set(chunk.reads.get() + 1);
    let state = &*chunk.state.borrow();
    let shown = match state {
        ChunkState::Pending { value, reason } => format!("pending {value:?} {reason:?}"),
        ChunkState::Blocked { value } => format!("blocked {value:?}"),
        ChunkState::Initialized { value } => format!("fulfilled {value}"),
    };
    format!("{} {shown} reads {}", chunk.name, chunk.reads.get())
}

fn main() {
    let chunk = Rc::new(ReactPromise::new(
        ChunkState::Pending {
            value: Some(vec![1, 2]),
            reason: Some("why".to_string()),
        },
        "a",
    ));
    println!("{}", status(&chunk));
    println!("{}", blocked_by_a_closure(&chunk));
    println!("{}", status(&chunk));
    println!("{}", blocked_by_a_closure(&chunk));
    written_by_a_closure(&chunk);
    println!("{}", status(&chunk));
    lent_by_a_closure(&chunk);
    println!("{}", status(&chunk));
}
