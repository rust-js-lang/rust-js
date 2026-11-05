// A flattened state written whole, of a variant whose fields read another
// place, `handler.value.get()`, or are made of variables, `Root { id,
// response }`: written in place, `chunk.value = handler.value;`, as
// upstream writes it, through a borrow and by its cell's `replace`; only a
// field that calls or reads the chunk itself is held before its tag is set
// (#784, #779, ADR 1044).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[derive(Clone, Copy)]
pub struct Root {
    id: u32,
    response: u32,
}

#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum ChunkState {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending { value: Option<u32> },
    #[cfg_attr(rust_js, rust_js::name = "resolved_model")]
    Resolved { value: u32, reason: Root },
    #[cfg_attr(rust_js, rust_js::name = "fulfilled")]
    Initialized { value: u32, reason: Option<u32> },
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

pub struct Handler {
    value: Cell<u32>,
    root: Cell<Option<u32>>,
}

fn resolve(chunk: &Chunk, value: u32, id: u32, response: u32) {
    *chunk.state.borrow_mut() = ChunkState::Resolved {
        value,
        reason: Root { id, response },
    };
}

fn fulfill(chunk: &Chunk, handler: &Handler) {
    let root = handler.root.get();
    *chunk.state.borrow_mut() = ChunkState::Initialized {
        value: handler.value.get(),
        reason: root,
    };
}

fn fulfill_replaced(chunk: &Chunk, handler: &Handler) -> bool {
    let old = chunk.state.replace(ChunkState::Initialized {
        value: handler.value.get(),
        reason: None,
    });
    matches!(old, ChunkState::Resolved { .. })
}

fn describe(chunk: &Chunk) -> String {
    match &*chunk.state.borrow() {
        ChunkState::Pending { value } => format!("pending {value:?}"),
        ChunkState::Resolved { value, reason } => {
            format!("resolved {value} {} {}", reason.id, reason.response)
        }
        ChunkState::Initialized { value, reason } => format!("fulfilled {value} {reason:?}"),
    }
}

fn main() {
    let chunk = Rc::new(ReactPromise::new(ChunkState::Pending { value: None }));
    resolve(&chunk, 3, 7, 9);
    println!("{}", describe(&chunk));
    let handler = Handler {
        value: Cell::new(5),
        root: Cell::new(Some(2)),
    };
    fulfill(&chunk, &handler);
    println!("{}", describe(&chunk));
    resolve(&chunk, 4, 8, 1);
    handler.value.set(6);
    println!("{} {}", fulfill_replaced(&chunk, &handler), describe(&chunk));
}
