// An or-pattern whose alternatives bind fields at one JS place (rd430):
// a tagged enum's `Circle { r } | Square { r }`, each `shape.r`, and a
// flattened state's `Pending { value } | Blocked { value }`, each
// `chunk.value`, as React's Flight client's `case PENDING: case BLOCKED:`
// reads one property: one test of either, the arm once, read and written
// through. Of fields at other places, `Circle { r } | Ring(r)`, and an
// integer's arms that do alike, each its own or one, as they do.
use std::cell::RefCell;
use std::rc::Rc;

pub enum Shape {
    Circle { r: f64 },
    Square { r: f64 },
    Ring(f64),
    Dot,
}

pub fn grow(shape: &mut Shape) {
    if let Shape::Circle { r } | Shape::Square { r } = shape {
        *r += 1.0;
    }
}

pub fn size(shape: &Shape) -> f64 {
    match shape {
        Shape::Circle { r } | Shape::Square { r } => *r,
        Shape::Dot => 0.0,
        Shape::Ring(r) => -r,
    }
}

pub fn apart(shape: &Shape) -> f64 {
    match shape {
        Shape::Circle { r } | Shape::Ring(r) => *r,
        _ => 1.0,
    }
}

pub fn weight(n: u32) -> u32 {
    let mut w = 0;
    match n {
        1 => w += 10,
        2 => w += 10,
        3 => w += 30,
        _ => {}
    }
    w
}

#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum ChunkState {
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

pub fn listen(chunk: &Rc<ReactPromise>, id: u32) {
    if let ChunkState::Pending { value } | ChunkState::Blocked { value } = &mut *chunk.state.borrow_mut() {
        match value {
            None => *value = Some(vec![id]),
            Some(listeners) => listeners.push(id),
        }
    }
}

pub fn take(chunk: &Rc<ReactPromise>, text: String) -> Option<Vec<u32>> {
    let listeners = match &mut *chunk.state.borrow_mut() {
        ChunkState::Pending { value } | ChunkState::Blocked { value } => value.take(),
        _ => None,
    };
    *chunk.state.borrow_mut() = ChunkState::Initialized { value: text };
    listeners
}

pub fn read(chunk: &Rc<ReactPromise>) -> String {
    match &*chunk.state.borrow() {
        ChunkState::Pending { value } | ChunkState::Blocked { value } => format!("waiting {value:?}"),
        ChunkState::Initialized { value } => format!("done {value}"),
    }
}

fn main() {
    for kind in 0..4 {
        let mut shape = match kind {
            0 => Shape::Circle { r: 2.0 },
            1 => Shape::Square { r: 3.0 },
            2 => Shape::Ring(4.0),
            _ => Shape::Dot,
        };
        grow(&mut shape);
        println!("{} {} {}", size(&shape), apart(&shape), weight(kind));
    }
    let chunk = Rc::new(ReactPromise::new(ChunkState::Pending { value: None }));
    println!("{}", read(&chunk));
    listen(&chunk, 1);
    listen(&chunk, 2);
    println!("{}", read(&chunk));
    let waiting = take(&chunk, "x".to_string());
    *chunk.state.borrow_mut() = ChunkState::Blocked { value: waiting };
    listen(&chunk, 3);
    println!("{}", read(&chunk));
    let woken = take(&chunk, "done".to_string());
    println!("{woken:?}");
    println!("{}", read(&chunk));
    listen(&chunk, 4);
    let none = take(&chunk, "again".to_string());
    println!("{none:?} {}", read(&chunk));
}
