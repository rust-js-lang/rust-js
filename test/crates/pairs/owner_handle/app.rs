//! A crate whose callbacks, made before the library's request they need,
//! read it through a cell set once it's made, as React's Fizz server's
//! entries do: the library's contract says the request alone owns them, so
//! the cell is the request's `let` (ADR 0317, amended).
use std::cell::OnceCell;
use std::rc::{Rc, Weak};

use dep::{Request, create_request, start_flowing, start_work};

fn render_to_pipeable_stream(name: &str) -> Rc<Request> {
    let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
    let on_all_ready: Rc<dyn Fn()> = {
        let cell = Rc::clone(&request_cell);
        Rc::new(move || {
            let request = cell.get().and_then(Weak::upgrade).unwrap();
            start_flowing(&request, "all");
        })
    };
    let on_shell_ready: Rc<dyn Fn()> = {
        let cell = Rc::clone(&request_cell);
        Rc::new(move || match cell.get().and_then(Weak::upgrade) {
            Some(request) => start_flowing(&request, "shell"),
            None => println!("the shell is ready before the request is"),
        })
    };
    let request = create_request(name, Some(on_all_ready), Some(on_shell_ready));
    let _ = request_cell.set(Rc::downgrade(&request));
    request
}

pub fn main() {
    let request = render_to_pipeable_stream("a");
    start_work(&request);
    start_work(&request);
    println!("{}: {:?}", request.name, request.log.borrow());
}
