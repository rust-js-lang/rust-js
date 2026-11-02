// React's Flight client's initializeModelChunk, of its nullable
// thread-locals: a guard holds the handler it took and the chunk it read,
// each `null` or a value, and restores them as they were, however the
// scope is left, one where it was tracked: `finally { initializingHandler =
// prevHandler; if (..) { initializingChunk = prevChunk; } }`, as upstream,
// where each was restored `?? null` (rd523).
use std::cell::RefCell;
use std::rc::Rc;

pub struct Chunk {
    pub id: u32,
}

thread_local! {
    #[cfg_attr(rust_js, rust_js::nullable)]
    static INITIALIZING_HANDLER: RefCell<Option<Rc<Chunk>>> = const { RefCell::new(None) };
    #[cfg_attr(rust_js, rust_js::nullable)]
    static INITIALIZING_CHUNK: RefCell<Option<Rc<Chunk>>> = const { RefCell::new(None) };
}

struct RestoreInitializing {
    prev_handler: Option<Rc<Chunk>>,
    prev_chunk: Option<Rc<Chunk>>,
    tracked: bool,
}

impl Drop for RestoreInitializing {
    fn drop(&mut self) {
        INITIALIZING_HANDLER.set(self.prev_handler.take());
        if self.tracked {
            INITIALIZING_CHUNK.set(self.prev_chunk.take());
        }
    }
}

fn initialize(chunk: &Rc<Chunk>, tracked: bool, run: &dyn Fn() -> u32) -> u32 {
    let prev_handler = INITIALIZING_HANDLER.take();
    let prev_chunk = INITIALIZING_CHUNK.with_borrow(Clone::clone);
    if tracked {
        INITIALIZING_CHUNK.set(Some(Rc::clone(chunk)));
    }
    let _restore = RestoreInitializing {
        prev_handler,
        prev_chunk,
        tracked,
    };
    run()
}

fn current() -> String {
    let handler = INITIALIZING_HANDLER.with_borrow(|h| h.as_ref().map(|h| h.id));
    let chunk = INITIALIZING_CHUNK.with_borrow(|c| c.as_ref().map(|c| c.id));
    format!("{handler:?} {chunk:?}")
}

fn main() {
    let root = Rc::new(Chunk { id: 1 });
    let inner = Rc::new(Chunk { id: 2 });
    println!("{} {}", initialize(&root, true, &|| 10), current());
    INITIALIZING_HANDLER.set(Some(Rc::clone(&root)));
    INITIALIZING_CHUNK.set(Some(Rc::clone(&root)));
    let seen = initialize(&inner, true, &|| INITIALIZING_CHUNK.with_borrow(|c| c.as_ref().map_or(0, |c| c.id)));
    println!("{seen} {}", current());
    let caught = std::panic::catch_unwind(|| initialize(&inner, false, &|| panic!("boom")));
    println!("{} {}", caught.is_err(), current());
}
