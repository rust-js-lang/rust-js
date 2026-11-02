// A callback that sets its cell itself, before the request is made: the
// maker's `set` is `Err` then, and the callback reads what it set, a
// `Weak` to nothing, as Rust's `OnceCell` keeps it. Read but by `get()
// .and_then(Weak::upgrade)`, the cell is a cell, as written (ADR 0317,
// amended).

use std::cell::OnceCell;
use std::rc::{Rc, Weak};

struct Request {
    name: &'static str,
    on_ready: Rc<dyn Fn()>,
}

fn create_request(name: &'static str, on_ready: Rc<dyn Fn()>) -> Rc<Request> {
    let request = Rc::new(Request { name, on_ready });
    complete(&request);
    request
}

fn complete(request: &Request) {
    let on_ready = &request.on_ready;
    on_ready();
}

fn render(name: &'static str) -> Rc<Request> {
    let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
    let on_ready: Rc<dyn Fn()> = {
        let cell = Rc::clone(&request_cell);
        Rc::new(move || {
            let _ = cell.set(Weak::new());
            match cell.get().and_then(Weak::upgrade) {
                Some(request) => println!("{} is ready", request.name),
                None => println!("no request"),
            }
        })
    };
    let request = create_request(name, on_ready);
    let _ = request_cell.set(Rc::downgrade(&request));
    request
}

fn main() {
    let request = render("a");
    complete(&request);
}
