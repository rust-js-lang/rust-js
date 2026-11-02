//! A library's request whose callback is a public field, as React's Flight
//! server's `on_all_ready` was (rd553): another crate may clone it out and
//! keep it past the request, so its return doesn't alone own what it's
//! given, and its contract says none (ADR 0317, amended).
use std::rc::Rc;

pub struct Request {
    pub name: String,
    pub on_all_ready: Rc<dyn Fn()>,
}

pub fn create_prerender_request(name: &str, on_all_ready: Rc<dyn Fn()>) -> Rc<Request> {
    Rc::new(Request {
        name: name.to_string(),
        on_all_ready,
    })
}

pub fn start_work(request: &Rc<Request>) {
    let on_all_ready = &request.on_all_ready;
    on_all_ready();
}
