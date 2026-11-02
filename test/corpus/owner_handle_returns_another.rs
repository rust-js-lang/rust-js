// A call that keeps the request its callback is given to and returns
// another: the callback runs while the request returned may be gone, and
// its upgrade, of the one returned, gives `None` once it's dropped, as Rust
// counts it. What it returns doesn't own the callback, so the request is
// counted, and its cell a cell, as written (ADR 0317, amended).

use std::cell::{OnceCell, RefCell};
use std::rc::{Rc, Weak};

struct Request {
    name: &'static str,
    on_ready: Rc<dyn Fn()>,
}

fn noop() {}

fn create_request(name: &'static str, on_ready: Rc<dyn Fn()>, kept: &RefCell<Vec<Rc<Request>>>) -> Rc<Request> {
    let made = Rc::new(Request { name: "kept", on_ready });
    kept.borrow_mut().push(Rc::clone(&made));
    Rc::new(Request {
        name,
        on_ready: Rc::new(noop),
    })
}

fn complete(request: &Request) {
    let on_ready = &request.on_ready;
    on_ready();
}

fn render(name: &'static str, kept: &RefCell<Vec<Rc<Request>>>) -> Rc<Request> {
    let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
    let on_ready: Rc<dyn Fn()> = {
        let cell = Rc::clone(&request_cell);
        Rc::new(move || match cell.get().and_then(Weak::upgrade) {
            Some(request) => println!("{} is ready", request.name),
            None => println!("no request"),
        })
    };
    let request = create_request(name, on_ready, kept);
    let _ = request_cell.set(Rc::downgrade(&request));
    request
}

fn main() {
    let kept = RefCell::new(Vec::new());
    let request = render("a", &kept);
    kept.borrow().iter().for_each(|made| complete(made));
    drop(request);
    kept.borrow().iter().for_each(|made| complete(made));
}
