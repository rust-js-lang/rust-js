// A callback's cell set twice: the second `set` is `Err`, giving back what
// it was given, and the callback reads the first, as Rust's `OnceCell`
// does. Set more than once, the cell is a cell, as written (ADR 0317,
// amended).

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

fn render(name: &'static str, other: &Rc<Request>) -> Rc<Request> {
    let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
    let on_ready: Rc<dyn Fn()> = {
        let cell = Rc::clone(&request_cell);
        Rc::new(move || match cell.get().and_then(Weak::upgrade) {
            Some(request) => println!("{} is ready", request.name),
            None => println!("no request"),
        })
    };
    let request = create_request(name, on_ready);
    let _ = request_cell.set(Rc::downgrade(&request));
    let second = request_cell.set(Rc::downgrade(other));
    println!("set again: {}", second.is_err());
    request
}

fn main() {
    let other = create_request("other", Rc::new(|| println!("other is ready")));
    let request = render("a", &other);
    complete(&request);
    complete(&other);
}
