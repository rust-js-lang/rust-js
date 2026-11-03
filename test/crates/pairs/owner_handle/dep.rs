//! A library's request, as React's Fizz server's, which owns the callbacks
//! it's given alone and calls them only through itself: its contract says
//! `create_request`'s return owns them (ADR 0317, amended).
use std::cell::{Cell, RefCell};
use std::rc::Rc;

pub struct Request {
    pub name: String,
    pub log: RefCell<Vec<String>>,
    flowed: Cell<u32>,
    on_all_ready: Rc<dyn Fn()>,
    on_shell_ready: Rc<dyn Fn()>,
}

fn noop() {}

fn request_instance(
    name: &str,
    on_all_ready: Option<Rc<dyn Fn()>>,
    on_shell_ready: Option<Rc<dyn Fn()>>,
) -> Rc<Request> {
    Rc::new(Request {
        name: name.to_string(),
        log: RefCell::new(Vec::new()),
        flowed: Cell::new(0),
        on_all_ready: on_all_ready.unwrap_or_else(|| Rc::new(noop)),
        on_shell_ready: on_shell_ready.unwrap_or_else(|| Rc::new(noop)),
    })
}

pub fn create_request(
    name: &str,
    on_all_ready: Option<Rc<dyn Fn()>>,
    on_shell_ready: Option<Rc<dyn Fn()>>,
) -> Rc<Request> {
    let request = request_instance(name, on_all_ready, on_shell_ready);
    complete_shell(&request);
    request
}

/// One that completes nothing before it returns: nothing it runs may call
/// what it's given, which its contract says too (rd621).
pub fn create_quiet_request(
    name: &str,
    on_all_ready: Option<Rc<dyn Fn()>>,
    on_shell_ready: Option<Rc<dyn Fn()>>,
) -> Rc<Request> {
    let request = request_instance(name, on_all_ready, on_shell_ready);
    note(&request, "made");
    request
}

fn note(request: &Request, what: &str) {
    request.log.borrow_mut().push(format!("{} {what}", request.name));
}

fn complete_shell(request: &Rc<Request>) {
    let on_shell_ready = &request.on_shell_ready;
    on_shell_ready();
}

pub fn start_work(request: &Rc<Request>) {
    complete_shell(request);
    let on_all_ready = &request.on_all_ready;
    on_all_ready();
}

/// A request whose callback another crate may clone: its return doesn't
/// alone own it.
pub struct Open {
    pub on_ready: Rc<dyn Fn()>,
}

pub fn open_request(on_ready: Rc<dyn Fn()>) -> Rc<Open> {
    Rc::new(Open { on_ready })
}

pub fn start_flowing(request: &Request, destination: &str) {
    request.flowed.set(request.flowed.get() + 1);
    request
        .log
        .borrow_mut()
        .push(format!("{destination} {}", request.flowed.get()));
}
