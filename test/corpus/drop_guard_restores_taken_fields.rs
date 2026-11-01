// React's Flight client's initializeModelChunk: a guard holds the handler
// and the chunk it found, each taken back as it's dropped, `prev_handler.
// take()`, and restores them however the scope is left. Each take is of a
// field nothing reads after, though the other field is, so the guard is
// its fields, the variable each was made of or a `const` of its own name;
// and the code after the call, an `if` of only comments too, is in the one
// `try` whose `finally` restores them, as upstream's `const prevHandler =
// initializingHandler; try { .. } finally { initializingHandler =
// prevHandler; }`, where each was restored in a `catch` and again after
// (rd429).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Handler {
    pub errored: Cell<bool>,
    pub deps: Cell<u32>,
}

thread_local! {
    static INITIALIZING_HANDLER: RefCell<Option<Rc<Handler>>> = const { RefCell::new(None) };
    static INITIALIZING_CHUNK: RefCell<Option<Rc<String>>> = const { RefCell::new(None) };
}

const DEV: bool = true;

struct RestoreInitializing {
    prev_handler: Option<Rc<Handler>>,
    prev_chunk: Option<Rc<String>>,
}

impl Drop for RestoreInitializing {
    fn drop(&mut self) {
        INITIALIZING_HANDLER.set(self.prev_handler.take());
        if DEV {
            INITIALIZING_CHUNK.set(self.prev_chunk.take());
        }
    }
}

fn parse_model(text: &str) -> u32 {
    text.parse().unwrap()
}

fn initialize_model_chunk(chunk: &str) -> u32 {
    let _restore = RestoreInitializing {
        prev_handler: INITIALIZING_HANDLER.take(),
        prev_chunk: INITIALIZING_CHUNK.with_borrow(Clone::clone),
    };
    INITIALIZING_CHUNK.set(Some(Rc::new(chunk.to_owned())));
    let value = parse_model(chunk);
    if let Some(handler) = INITIALIZING_HANDLER.with_borrow(Clone::clone) {
        if handler.errored.get() {
            // Ignore error parsing debug info, we'll report the original error instead.
        } else if handler.deps.get() > 0 {
            // TODO: Block the resolution of the error until all the debug info has loaded.
        }
    }
    value
}

fn state() -> String {
    let handler = INITIALIZING_HANDLER.with_borrow(|h| h.as_ref().map(|h| h.deps.get()));
    let chunk = INITIALIZING_CHUNK.with_borrow(Clone::clone);
    format!("{handler:?} {chunk:?}")
}

fn main() {
    INITIALIZING_HANDLER.set(Some(Rc::new(Handler { errored: Cell::new(false), deps: Cell::new(2) })));
    INITIALIZING_CHUNK.set(Some(Rc::new("root".to_owned())));
    println!("{} {}", initialize_model_chunk("41"), state());
    let caught = std::panic::catch_unwind(|| initialize_model_chunk("x"));
    println!("{} {}", caught.is_err(), state());
}
