// A request's variable swapped with another's after its callback's cell is
// set: the cell still holds the first, which the callback reads, where the
// variable holds the other. Changed through a `&mut`, the variable is no
// cell, which is a cell, as written (ADR 0317, amended).

use std::cell::OnceCell;
use std::mem;
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

fn render(name: &'static str, other: &mut Rc<Request>) -> Rc<Request> {
    let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
    let on_ready: Rc<dyn Fn()> = {
        let cell = Rc::clone(&request_cell);
        Rc::new(move || match cell.get().and_then(Weak::upgrade) {
            Some(request) => println!("{} is ready", request.name),
            None => println!("no request"),
        })
    };
    let mut request = create_request(name, on_ready);
    let _ = request_cell.set(Rc::downgrade(&request));
    mem::swap(&mut request, other);
    request
}

fn main() {
    let mut other = create_request("other", Rc::new(|| println!("other is ready")));
    let request = render("a", &mut other);
    println!("{} {}", request.name, other.name);
    complete(&request);
    complete(&other);
}
