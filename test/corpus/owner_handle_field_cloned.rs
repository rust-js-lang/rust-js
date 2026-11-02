// A callback its request gives a clone of, which may outlive it: the field
// that holds it is read but to call through it, so the request doesn't
// alone own it. Called after the request is dropped, its upgrade gives
// `None`, as Rust counts it. The request is counted, and its cell a cell,
// as written (ADR 0317, amended).

use std::cell::OnceCell;
use std::rc::{Rc, Weak};

struct Request {
    name: &'static str,
    on_ready: Rc<dyn Fn()>,
}

impl Request {
    fn on_ready(&self) -> Rc<dyn Fn()> {
        Rc::clone(&self.on_ready)
    }
}

fn create_request(name: &'static str, on_ready: Rc<dyn Fn()>) -> Rc<Request> {
    Rc::new(Request { name, on_ready })
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
    let _ = request_cell.set(Rc::downgrade(&request));
    request
}

fn main() {
    let request = render("a");
    let on_ready = request.on_ready();
    on_ready();
    drop(request);
    on_ready();
}
