// A callback the call that makes the request keeps apart from it too, in a
// list of its own, by a function it gives it on to: what it returns doesn't
// alone own it, so it may run after the request is dropped, where its
// upgrade gives `None`, as Rust counts it. The request is counted, and its
// cell a cell, as written (ADR 0317, amended).

use std::cell::{OnceCell, RefCell};
use std::rc::{Rc, Weak};

struct Request {
    name: &'static str,
    on_ready: Rc<dyn Fn()>,
}

thread_local! {
    static READY: RefCell<Vec<Rc<dyn Fn()>>> = const { RefCell::new(Vec::new()) };
}

fn request_instance(name: &'static str, on_ready: Rc<dyn Fn()>) -> Rc<Request> {
    READY.with(|ready| ready.borrow_mut().push(Rc::clone(&on_ready)));
    Rc::new(Request { name, on_ready })
}

fn create_request(name: &'static str, on_ready: Rc<dyn Fn()>) -> Rc<Request> {
    request_instance(name, on_ready)
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
    complete(&request);
    drop(request);
    READY.with(|ready| {
        for on_ready in ready.borrow().iter() {
            on_ready();
        }
    });
}
