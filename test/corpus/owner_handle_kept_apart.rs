// A callback whose request may be gone when it runs: a clone of it kept
// apart from the request that's given it, called after the request is
// dropped. Its upgrade gives `None` there, as Rust counts it, so the
// request is counted, and its cell a cell, as written (ADR 0317, amended).

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

fn render(name: &'static str) -> (Rc<Request>, Rc<dyn Fn()>) {
    let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
    let on_ready: Rc<dyn Fn()> = {
        let cell = Rc::clone(&request_cell);
        Rc::new(move || match cell.get().and_then(Weak::upgrade) {
            Some(request) => println!("{} is ready", request.name),
            None => println!("no request"),
        })
    };
    let kept = Rc::clone(&on_ready);
    let request = create_request(name, on_ready);
    let _ = request_cell.set(Rc::downgrade(&request));
    (request, kept)
}

fn main() {
    let (request, kept) = render("a");
    complete(&request);
    kept();
    drop(request);
    kept();
}
