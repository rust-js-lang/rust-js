//! A crate whose callback reads the library's request through a cell set
//! once it's made, as React's Flight server's prerender entries do. The
//! request's field is public, so the crate clones the callback out and
//! calls it after the request is gone, when the cell's `Weak` gives `None`,
//! where the request's `let` would still hold it: the cell is counted, its
//! `dyn Fn` drops it, and the library, whose `dyn Fn`s drop nothing, would
//! drop nothing of it, so the crate is refused (ADRs 0317, 0626, amended).
use std::cell::OnceCell;
use std::rc::{Rc, Weak};

use dep::{Request, create_prerender_request, start_work};

fn prerender(name: &str) -> Rc<Request> {
    let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
    let on_all_ready = {
        let cell = Rc::clone(&request_cell);
        move || match cell.get().and_then(Weak::upgrade) {
            Some(request) => println!("{} is ready", request.name),
            None => println!("no request"),
        }
    };
    let request = create_prerender_request(name, Rc::new(on_all_ready));
    let _ = request_cell.set(Rc::downgrade(&request));
    request
}

pub fn main() {
    let request = prerender("a");
    start_work(&request);
    let kept = Rc::clone(&request.on_all_ready);
    drop(request);
    kept();
}
