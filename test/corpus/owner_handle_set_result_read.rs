// A callback's cell set once whose `Result` is read: `Ok`, as Rust's
// `OnceCell` gives it. Its `Result` read, the cell is a cell, as written
// (ADR 0317, amended).

use std::cell::OnceCell;
use std::rc::{Rc, Weak};

struct Request {
    name: &'static str,
    on_ready: Rc<dyn Fn()>,
}

fn create_request(name: &'static str, on_ready: Rc<dyn Fn()>) -> Rc<Request> {
    Rc::new(Request { name, on_ready })
}

fn complete(request: &Request) {
    let on_ready = &request.on_ready;
    on_ready();
}

fn render(name: &'static str) -> Rc<Request> {
    let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
    let on_ready: Rc<dyn Fn()> = {
        let cell = Rc::clone(&request_cell);
        Rc::new(move || match cell.get().and_then(Weak::upgrade) {
            Some(request) => println!("{} is ready", request.name),
            None => println!("no request"),
        })
    };
    let request = create_request(name, on_ready);
    let set = request_cell.set(Rc::downgrade(&request));
    println!("set: {}", set.is_ok());
    request
}

fn main() {
    let request = render("a");
    complete(&request);
}
