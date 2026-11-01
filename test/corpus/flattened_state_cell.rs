// A class whose state is a discriminated union flattened into it, as React's
// Flight client's chunk has its `status`, `value` and `reason` on itself
// (rd380, ADR 1044): its constructor takes the state's keys, each `new`
// gives them, and the state is read, changed in a variant's field, and
// replaced by another variant in place, the class's own fields kept.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum ChunkState {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending {
        value: Option<Vec<u32>>,
        reason: Option<Vec<u32>>,
    },
    #[cfg_attr(rust_js, rust_js::name = "fulfilled")]
    Initialized { value: String, reason: Option<String> },
    #[cfg_attr(rust_js, rust_js::name = "rejected")]
    Errored { reason: String },
    #[cfg_attr(rust_js, rust_js::name = "halted")]
    Halted,
}

#[cfg_attr(rust_js, rust_js::class)]
pub struct ReactPromise {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub state: RefCell<ChunkState>,
    pub reads: Cell<u32>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl ReactPromise {
    fn new(state: ChunkState) -> Self {
        ReactPromise {
            state: RefCell::new(state),
            reads: Cell::new(0),
        }
    }
}

type Chunk = Rc<ReactPromise>;

fn listen(chunk: &Chunk, id: u32) {
    if let ChunkState::Pending { value, .. } = &mut *chunk.state.borrow_mut() {
        value.get_or_insert_with(Vec::new).push(id);
    }
}

fn wake(chunk: &Chunk, value: &str) -> Vec<u32> {
    let listeners = match &mut *chunk.state.borrow_mut() {
        ChunkState::Pending { value, .. } => value.take(),
        _ => None,
    };
    *chunk.state.borrow_mut() = ChunkState::Initialized {
        value: value.to_string(),
        reason: None,
    };
    listeners.unwrap_or_default()
}

fn fail(chunk: &Chunk, reason: String) {
    let next = ChunkState::Errored { reason };
    *chunk.state.borrow_mut() = next;
}

// Written whole through a `&mut` to it, which is the chunk itself.
fn halt(state: &mut ChunkState) {
    *state = ChunkState::Halted;
}

// Its two fields written through one borrow a let chain holds, as the
// Flight client's initializeModelChunk puts its deferred listeners back:
// each the chunk's own key (rd533).
fn requeue(chunk: &Chunk, deferred: Option<Vec<u32>>, rejects: Option<Vec<u32>>) {
    if deferred.is_some()
        && let ChunkState::Pending { value, reason } = &mut *chunk.state.borrow_mut()
    {
        *value = deferred;
        *reason = rejects;
    }
}

fn read(chunk: &Chunk) -> String {
    chunk.reads.set(chunk.reads.get() + 1);
    match &*chunk.state.borrow() {
        ChunkState::Pending { value, reason } => {
            format!("pending {:?} {:?}", value, reason)
        }
        ChunkState::Initialized { value, reason } => format!("fulfilled {value} {reason:?}"),
        ChunkState::Errored { reason } => format!("rejected {reason}"),
        ChunkState::Halted => "halted".to_string(),
    }
}

fn made(state: ChunkState) -> Chunk {
    Rc::new(ReactPromise::new(state))
}

fn main() {
    let chunk = made(ChunkState::Pending {
        value: None,
        reason: None,
    });
    println!("{}", read(&chunk));
    requeue(&chunk, None, Some(vec![9]));
    requeue(&chunk, Some(vec![7]), Some(vec![8]));
    println!("{}", read(&chunk));
    requeue(&chunk, Some(vec![]), None);
    listen(&chunk, 1);
    listen(&chunk, 2);
    println!("{}", read(&chunk));
    println!("{:?}", wake(&chunk, "done"));
    println!("{}", read(&chunk));
    // Already fulfilled: nothing to wake, its value replaced.
    println!("{:?}", wake(&chunk, "again"));
    println!("{}", read(&chunk));
    fail(&chunk, "boom".to_string());
    println!("{}", read(&chunk));
    println!("reads {}", chunk.reads.get());
    halt(&mut chunk.state.borrow_mut());
    println!("{} {}", read(&chunk), chunk.reads.get());
    let halted = made(ChunkState::Halted);
    println!("{}", read(&halted));
    let given = ChunkState::Errored {
        reason: "given".to_string(),
    };
    let errored = made(given);
    println!("{}", read(&errored));
}
