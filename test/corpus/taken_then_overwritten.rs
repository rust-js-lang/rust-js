// A `take()`'s `None`, written over before anything could see it, is never
// written (rd524): React's Flight client takes a chunk's listeners and
// makes it fulfilled, `const listeners = chunk.value; chunk.status =
// 'fulfilled'; chunk.value = text;`, with no `chunk.value = null` between.
// Where something between could see it, a call given the chunk, a read of
// the chunk, a write of the field through what it holds, it's written.

use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug)]
#[cfg_attr(rust_js, rust_js::tag = "status")]
enum ChunkState {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending {
        #[cfg_attr(rust_js, rust_js::nullable)]
        value: Option<Vec<u32>>,
    },
    #[cfg_attr(rust_js, rust_js::name = "blocked")]
    Blocked {
        #[cfg_attr(rust_js, rust_js::nullable)]
        value: Option<Vec<u32>>,
    },
    #[cfg_attr(rust_js, rust_js::name = "fulfilled")]
    Initialized { value: String },
}

#[cfg_attr(rust_js, rust_js::class)]
struct ReactPromise {
    #[cfg_attr(rust_js, rust_js::flatten)]
    state: RefCell<ChunkState>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl ReactPromise {
    fn new(state: ChunkState) -> Self {
        ReactPromise {
            state: RefCell::new(state),
        }
    }
}

fn listen(chunk: &Rc<ReactPromise>, id: u32) {
    if let ChunkState::Pending { value } | ChunkState::Blocked { value } = &mut *chunk.state.borrow_mut() {
        match value {
            None => *value = Some(vec![id]),
            Some(listeners) => listeners.push(id),
        }
    }
}

// Upstream's `const listeners = chunk.value; chunk.status = 'fulfilled';
// chunk.value = text;`.
fn fulfill(chunk: &Rc<ReactPromise>, text: String) -> Option<Vec<u32>> {
    let listeners = match &mut *chunk.state.borrow_mut() {
        ChunkState::Pending { value } | ChunkState::Blocked { value } => value.take(),
        _ => None,
    };
    *chunk.state.borrow_mut() = ChunkState::Initialized { value: text };
    listeners
}

fn shown(chunk: &ReactPromise) -> String {
    format!("{:?}", chunk.state.borrow())
}

// The chunk shown between: its `None` written first.
fn fulfill_shown(chunk: &Rc<ReactPromise>, text: String) -> (Option<Vec<u32>>, String) {
    let listeners = match &mut *chunk.state.borrow_mut() {
        ChunkState::Pending { value } | ChunkState::Blocked { value } => value.take(),
        _ => None,
    };
    let between = shown(chunk);
    *chunk.state.borrow_mut() = ChunkState::Initialized { value: text };
    (listeners, between)
}

struct Slot {
    value: Option<u32>,
    count: u32,
    seen: String,
}

// A plain struct's field, taken and set again, another field set between.
fn replaced(slot: &mut Slot, value: u32) -> Option<u32> {
    let old = slot.value.take();
    slot.count = value;
    slot.value = Some(value);
    old
}

// Another field set to what's read of the taken one: its `None`.
fn read_between(slot: &mut Slot, value: u32) -> Option<u32> {
    let old = slot.value.take();
    slot.seen = format!("{:?}", slot.value);
    slot.value = Some(value);
    old
}

// Only another field set after: the `None` stays.
fn emptied(slot: &mut Slot, value: u32) -> Option<u32> {
    let old = slot.value.take();
    slot.count = value;
    old
}

// Read in its branch, after it's taken: its `None` written first.
fn branch_read(slot: &mut Slot, taken: bool) -> Option<u32> {
    let mut old = None;
    if taken {
        old = slot.value.take();
        slot.seen = format!("{:?}", slot.value);
    }
    slot.value = Some(8);
    old
}

// What's written after is another's: its `None` stays.
fn moved_on<'a>(mut slot: &'a mut Slot, other: &'a mut Slot) -> Option<u32> {
    let old = slot.value.take();
    slot = other;
    slot.value = Some(9);
    old
}

// Two writes the program makes, each kept, as written.
fn twice(slot: &mut Slot) {
    slot.count = 1;
    slot.count = 2;
}

fn main() {
    let chunk = Rc::new(ReactPromise::new(ChunkState::Pending { value: None }));
    listen(&chunk, 1);
    listen(&chunk, 2);
    println!("{:?} {}", fulfill(&chunk, "done".to_string()), shown(&chunk));
    let blocked = Rc::new(ReactPromise::new(ChunkState::Blocked { value: Some(vec![3]) }));
    println!("{:?}", fulfill_shown(&blocked, "late".to_string()));
    println!("{}", shown(&blocked));
    let mut slot = Slot {
        value: Some(4),
        count: 0,
        seen: String::new(),
    };
    println!("{:?} {:?} {}", replaced(&mut slot, 5), slot.value, slot.count);
    println!("{:?} {:?} {}", read_between(&mut slot, 6), slot.value, slot.seen);
    println!("{:?} {:?} {}", emptied(&mut slot, 7), slot.value, slot.count);
    slot.value = Some(10);
    println!("{:?} {:?} {}", branch_read(&mut slot, true), slot.value, slot.seen);
    let mut other = Slot {
        value: None,
        count: 0,
        seen: String::new(),
    };
    println!("{:?} {:?} {:?}", moved_on(&mut slot, &mut other), slot.value, other.value);
    twice(&mut slot);
    println!("{}", slot.count);
}
