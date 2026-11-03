// Callbacks made before the request they need, read through a cell set once
// it's made. `create_request` calls none of them before it returns, but a
// body it runs names the field `on_shell_ready` is kept in, which is all
// that may call it, so that one isn't quiet, and the handle keeps each
// callback's `unwrap`, `on_all_ready`'s too (ADR 0317, amended, rd621).

use std::cell::{Cell, OnceCell, RefCell};
use std::rc::{Rc, Weak};

type Callback = dyn Fn();

struct Request {
    name: String,
    flowed: Cell<u32>,
    log: RefCell<Vec<String>>,
    on_all_ready: Rc<Callback>,
    on_shell_ready: Rc<Callback>,
}

fn noop() {}

// What a callback is, which it tells without calling it.
fn kind(_callback: &Callback) -> &'static str {
    "callback"
}

fn create_request(name: &str, on_all_ready: Option<Rc<Callback>>, on_shell_ready: Option<Rc<Callback>>) -> Rc<Request> {
    let request = Rc::new(Request {
        name: name.to_string(),
        flowed: Cell::new(0),
        log: RefCell::new(Vec::new()),
        on_all_ready: on_all_ready.unwrap_or_else(|| Rc::new(noop)),
        on_shell_ready: on_shell_ready.unwrap_or_else(|| Rc::new(noop)),
    });
    describe(&request);
    request
}

fn describe(request: &Request) {
    let shell = kind(&*request.on_shell_ready);
    request.log.borrow_mut().push(format!("shell {shell}"));
}

fn complete_shell(request: &Rc<Request>) {
    let on_shell_ready = &request.on_shell_ready;
    on_shell_ready();
}

fn complete_all(request: &Rc<Request>) {
    let on_all_ready = &request.on_all_ready;
    on_all_ready();
}

fn start_flowing(request: &Request, destination: &str) {
    request.flowed.set(request.flowed.get() + 1);
    request
        .log
        .borrow_mut()
        .push(format!("{destination} {}", request.flowed.get()));
}

fn render(name: &str) -> Rc<Request> {
    let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
    let on_all_ready: Rc<Callback> = {
        let cell = Rc::clone(&request_cell);
        Rc::new(move || {
            let request = cell.get().and_then(Weak::upgrade).unwrap();
            start_flowing(&request, "all");
        })
    };
    let on_shell_ready: Rc<Callback> = {
        let cell = Rc::clone(&request_cell);
        Rc::new(move || {
            let request = cell.get().and_then(Weak::upgrade).unwrap();
            start_flowing(&request, "shell");
        })
    };
    let request = create_request(name, Some(on_all_ready), Some(on_shell_ready));
    let _ = request_cell.set(Rc::downgrade(&request));
    request
}

fn main() {
    let a = render("a");
    complete_shell(&a);
    complete_all(&a);
    println!("{}: {:?} {}", a.name, a.log.borrow(), a.flowed.get());
}
