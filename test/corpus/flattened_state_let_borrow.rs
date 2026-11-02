// A class's flattened state borrowed by a `let` of the guard's deref, as
// React's Flight client's chunk is, `let state = &mut
// *chunk.state.borrow_mut();`, read, tested and replaced whole through it,
// `*state = ChunkState::Blocked { .. }`: each key written on the chunk
// itself, the class's own fields kept (rd432, ADRs 0863, 1044).
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

fn block(chunk: &Chunk) -> bool {
    let state = &mut *chunk.state.borrow_mut();
    if let ChunkState::Pending { value, .. } = state {
        let listeners = value.take();
        *state = ChunkState::Blocked { value: listeners };
        return true;
    }
    false
}

fn resolve(chunk: &Chunk, value: &str) -> Vec<u32> {
    let state = &mut *chunk.state.borrow_mut();
    let listeners = match state {
        ChunkState::Pending { value, .. } | ChunkState::Blocked { value } => value.take().unwrap_or_default(),
        ChunkState::Initialized { .. } => return Vec::new(),
    };
    *state = ChunkState::Initialized { value: value.to_string() };
    listeners
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
    println!("{}", block(&chunk));
    println!("{}", status(&chunk));
    println!("{}", block(&chunk));
    println!("{:?}", resolve(&chunk, "done"));
    println!("{}", status(&chunk));
    println!("{:?}", resolve(&chunk, "again"));
    println!("{}", status(&chunk));
    let other = Rc::new(ReactPromise::new(ChunkState::Pending { value: None, reason: None }, "b"));
    println!("{:?} {}", resolve(&other, "early"), status(&other));
}
