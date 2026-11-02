// A request that's `Clone`: its clone clones the callback it holds, which
// so outlives the request it was given to. Called after that's dropped,
// its upgrade gives `None`, as Rust counts it, so the request is counted,
// and its cell a cell, as written (ADR 0317, amended).

use std::cell::OnceCell;
use std::rc::{Rc, Weak};

#[derive(Clone)]
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
    let _ = request_cell.set(Rc::downgrade(&request));
    request
}

fn main() {
    let request = render("a");
    let copy = Request::clone(&request);
    complete(&copy);
    drop(request);
    complete(&copy);
}
