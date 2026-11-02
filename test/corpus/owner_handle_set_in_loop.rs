// A callback's cell made once and set on each turn of a loop, each turn's
// request given its own callback: the second `set` is `Err`, so every
// callback reads the first request, as Rust's `OnceCell` keeps it, where a
// variable assigned each turn would read the last. Set again, the cell is a
// cell, as written (ADR 0317, amended).

use std::cell::OnceCell;
use std::rc::{Rc, Weak};

struct Request {
    name: String,
    on_ready: Rc<dyn Fn()>,
}

fn create_request(name: String, on_ready: Rc<dyn Fn()>) -> Rc<Request> {
    Rc::new(Request { name, on_ready })
}

fn complete(request: &Request) {
    let on_ready = &request.on_ready;
    on_ready();
}

fn render(names: &[&str]) -> Vec<Rc<Request>> {
    let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
    let mut requests = Vec::new();
    for name in names {
        let on_ready: Rc<dyn Fn()> = {
            let cell = Rc::clone(&request_cell);
            Rc::new(move || match cell.get().and_then(Weak::upgrade) {
                Some(request) => println!("{} is ready", request.name),
                None => println!("no request"),
            })
        };
        let request = create_request(name.to_string(), on_ready);
        let _ = request_cell.set(Rc::downgrade(&request));
        requests.push(request);
    }
    requests
}

fn main() {
    let requests = render(&["a", "b", "c"]);
    for request in &requests {
        complete(request);
    }
}
