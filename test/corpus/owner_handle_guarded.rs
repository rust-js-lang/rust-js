// Callbacks made before the request they need, read through a cell set
// once it's made, beside what has a destructor: a guard the maker drops
// after the cell is set, one each callback drops, a request a panic stops
// making, and requests made in a loop, each its own `let` (ADR 0317,
// amended).

use std::cell::OnceCell;
use std::rc::{Rc, Weak};

struct Request {
    name: String,
    on_ready: Rc<dyn Fn()>,
}

struct Guard(&'static str);

impl Drop for Guard {
    fn drop(&mut self) {
        println!("guard {} dropped", self.0);
    }
}

fn create_request(name: &str, on_ready: Rc<dyn Fn()>) -> Rc<Request> {
    if name.is_empty() {
        panic!("no name");
    }
    Rc::new(Request {
        name: name.to_string(),
        on_ready,
    })
}

fn complete(request: &Request) {
    let on_ready = &request.on_ready;
    on_ready();
}

fn render(name: &str) -> Rc<Request> {
    let guard = Guard("outer");
    let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
    let on_ready: Rc<dyn Fn()> = {
        let cell = Rc::clone(&request_cell);
        Rc::new(move || {
            let guard = Guard("inner");
            let request = cell.get().and_then(Weak::upgrade).unwrap();
            println!("{} ready", request.name);
            drop(guard);
        })
    };
    let request = create_request(name, on_ready);
    let _ = request_cell.set(Rc::downgrade(&request));
    drop(guard);
    request
}

fn main() {
    let request = render("a");
    complete(&request);
    let failed = std::panic::catch_unwind(|| render(""));
    println!("{}", failed.is_err());
    for i in 0..2 {
        let request = render(if i == 0 { "x" } else { "y" });
        complete(&request);
    }
}
