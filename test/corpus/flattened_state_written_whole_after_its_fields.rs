// A flattened state written whole, `*chunk.state.borrow_mut() =
// ChunkState::Failed { message: message(&*error) }`, of a variant whose
// field computes, by its cell's `replace` too: Rust computes the value
// before it writes any of it, so where `thrown` panics, the chunk keeps
// its state; each key is set only once its fields are computed, `const
// message$1 = message(error)` (#779, ADR 1044). And beside a guard whose
// drop ends the function, where tidy reads what a `catch` holds again,
// its keys set on the chunk, never a setter of the variable that holds it
// (#779).
use std::any::Any;
use std::cell::RefCell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum ChunkState {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending { value: Option<u32> },
    #[cfg_attr(rust_js, rust_js::name = "fulfilled")]
    Initialized { value: u32 },
    #[cfg_attr(rust_js, rust_js::name = "rejected")]
    Errored { reason: Option<&'static str> },
    #[cfg_attr(rust_js, rust_js::name = "failed")]
    Failed { message: String },
}

impl ChunkState {
    fn is_unreadable(&self) -> bool {
        matches!(self, ChunkState::Pending { value: Some(2) })
    }
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

fn thrown(error: &dyn Any) -> Option<&'static str> {
    let message = error.downcast_ref::<&'static str>().copied();
    assert!(message != Some("unreadable"), "thrown");
    message
}

fn message(error: &dyn Any) -> String {
    thrown(error).unwrap_or("unknown").to_string()
}

fn initialize(chunk: &Chunk, fail: bool) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if fail {
            panic!("bad model");
        }
        if chunk.state.borrow().is_unreadable() {
            panic!("unreadable");
        }
        *chunk.state.borrow_mut() = ChunkState::Initialized { value: 7 };
    }));
    if let Err(error) = result {
        let failed_chunk = chunk;
        *failed_chunk.state.borrow_mut() = ChunkState::Failed {
            message: message(&*error),
        };
    }
}

fn initialize_replaced(chunk: &Chunk, fail: bool) {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if fail {
            panic!("bad model");
        }
        if chunk.state.borrow().is_unreadable() {
            panic!("unreadable");
        }
        *chunk.state.borrow_mut() = ChunkState::Initialized { value: 7 };
    }));
    if let Err(error) = result {
        let old = chunk.state.replace(ChunkState::Failed {
            message: message(&*error),
        });
        drop(old);
    }
}

struct Restore(&'static str);

impl Drop for Restore {
    fn drop(&mut self) {
        print!("restore {} ", self.0);
    }
}

fn initialize_restored(chunk: &Chunk, fail: bool) {
    let _restore = Restore("handler");
    let result = catch_unwind(AssertUnwindSafe(|| {
        if fail {
            panic!("bad model");
        }
        if chunk.state.borrow().is_unreadable() {
            panic!("unreadable");
        }
        *chunk.state.borrow_mut() = ChunkState::Initialized { value: 7 };
    }));
    if let Err(error) = result {
        let errored_chunk = chunk;
        *errored_chunk.state.borrow_mut() = ChunkState::Errored {
            reason: thrown(&*error),
        };
    }
}

fn describe(chunk: &Chunk) -> String {
    match &*chunk.state.borrow() {
        ChunkState::Pending { value } => format!("pending {value:?}"),
        ChunkState::Initialized { value } => format!("fulfilled {value}"),
        ChunkState::Errored { reason } => format!("rejected {reason:?}"),
        ChunkState::Failed { message } => format!("failed {message}"),
    }
}

fn main() {
    for initialized in [initialize, initialize_replaced, initialize_restored] {
        for fail in [false, true] {
            let chunk = Rc::new(ReactPromise::new(ChunkState::Pending { value: Some(1) }));
            initialized(&chunk, fail);
            println!("{}", describe(&chunk));
        }
        let chunk = Rc::new(ReactPromise::new(ChunkState::Pending { value: Some(2) }));
        let r = catch_unwind(AssertUnwindSafe(|| initialized(&chunk, false)));
        println!("| {} {}", r.is_err(), describe(&chunk));
    }
}
