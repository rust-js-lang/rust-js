// A class's flattened state, each of whose variants has fields, lent
// `&mut` to a function that writes it whole, `*state = ..`, or a field of
// its variant: each key written on the chunk itself, the class's other
// fields, `name` and `reads`, kept (#478, ADR 1044). Lent as the guard's
// deref, through a `let` of it, and a state that's no chunk's, a variable
// of its own, lent the same, against native Rust.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum ChunkState {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending {
        value: Option<Vec<u32>>,
        reason: Option<String>,
    },
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

fn finish(state: &mut ChunkState, value: &str) {
    *state = ChunkState::Initialized { value: value.to_string() };
}

fn listen(state: &mut ChunkState, id: u32) {
    if let ChunkState::Pending { value, .. } = state {
        value.get_or_insert_with(Vec::new).push(id);
    }
}

fn lent(chunk: &Chunk, value: &str) {
    finish(&mut chunk.state.borrow_mut(), value);
}

fn lent_through(chunk: &Chunk, id: u32) {
    let state = &mut *chunk.state.borrow_mut();
    listen(state, id);
    if id > 2 {
        finish(state, "through");
    }
}

fn shown(state: &ChunkState) -> String {
    match state {
        ChunkState::Pending { value, reason } => format!("pending {value:?} {reason:?}"),
        ChunkState::Initialized { value } => format!("fulfilled {value}"),
    }
}

fn status(chunk: &Chunk) -> String {
    chunk.reads.set(chunk.reads.get() + 1);
    format!("{} {} reads {}", chunk.name, shown(&chunk.state.borrow()), chunk.reads.get())
}

fn main() {
    let chunk = Rc::new(ReactPromise::new(
        ChunkState::Pending {
            value: None,
            reason: Some("why".to_string()),
        },
        "a",
    ));
    println!("{}", status(&chunk));
    lent_through(&chunk, 1);
    lent_through(&chunk, 2);
    println!("{}", status(&chunk));
    lent(&chunk, "x");
    println!("{}", status(&chunk));
    let other = Rc::new(ReactPromise::new(ChunkState::Pending { value: None, reason: None }, "b"));
    lent_through(&other, 3);
    println!("{}", status(&other));
    let mut own = ChunkState::Pending { value: None, reason: None };
    listen(&mut own, 7);
    println!("{}", shown(&own));
    finish(&mut own, "own");
    println!("{}", shown(&own));
}
