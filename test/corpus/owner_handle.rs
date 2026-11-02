// Callbacks made before the request they need, as React's Fizz server's
// entries make `onAllReady` and `onShellReady` before `createRequest`: each
// closes over a cell set once the request is made, which holds its `Weak`,
// so the request's `Rc`, which owns them, makes no cycle. What
// `create_request` returns alone owns them, so each runs only while the
// request is alive, and its upgrade gives it once the cell is set, `None`
// before: the cell is upstream's `let request`, which `request =
// createRequest(..)` assigns and the closures read (ADR 0317, amended). One
// called before the cell is set reads `None`, and its `unwrap` panics, as
// Rust's does. Those whose request may be gone when they run are counted,
// as before (`owner_handle_counted`).

use std::cell::{Cell, OnceCell, RefCell};
use std::panic::{AssertUnwindSafe, catch_unwind};
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

fn request_instance(
    name: &str,
    on_all_ready: Option<Rc<Callback>>,
    on_shell_ready: Option<Rc<Callback>>,
) -> Rc<Request> {
    Rc::new(Request {
        name: name.to_string(),
        flowed: Cell::new(0),
        log: RefCell::new(Vec::new()),
        on_all_ready: on_all_ready.unwrap_or_else(|| Rc::new(noop)),
        on_shell_ready: on_shell_ready.unwrap_or_else(|| Rc::new(noop)),
    })
}

// What a request completes before it's returned, before its caller can
// set the cell.
#[derive(Clone, Copy)]
enum Early {
    Nothing,
    Shell,
    All,
}

fn create_request(
    name: &str,
    on_all_ready: Option<Rc<Callback>>,
    on_shell_ready: Option<Rc<Callback>>,
    early: Early,
) -> Rc<Request> {
    let request = request_instance(name, on_all_ready, on_shell_ready);
    match early {
        Early::Nothing => {}
        Early::Shell => {
            complete_shell(&request);
            return request;
        }
        Early::All => complete_all(&request),
    }
    request.log.borrow_mut().push("made".to_string());
    request
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

fn render(name: &str, early: Early) -> Rc<Request> {
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
        Rc::new(move || match cell.get().and_then(Weak::upgrade) {
            Some(request) => start_flowing(&request, "shell"),
            None => println!("the shell is ready before the request is"),
        })
    };
    let request = create_request(name, Some(on_all_ready), Some(on_shell_ready), early);
    let _ = request_cell.set(Rc::downgrade(&request));
    request
}

// Only the shell's callback, the other the request's own.
fn render_shell(name: &str) -> Rc<Request> {
    let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
    let on_shell_ready: Rc<Callback> = {
        let cell = Rc::clone(&request_cell);
        Rc::new(move || {
            if let Some(request) = cell.get().and_then(Weak::upgrade) {
                start_flowing(&request, "shell only");
            }
        })
    };
    let request = create_request(name, None, Some(on_shell_ready), Early::Nothing);
    let _ = request_cell.set(Rc::downgrade(&request));
    request
}

fn main() {
    let a = render("a", Early::Nothing);
    complete_shell(&a);
    complete_all(&a);
    complete_all(&a);
    println!("{}: {:?} {}", a.name, a.log.borrow(), a.flowed.get());

    // Its shell completed before its cell is set: `None` there.
    let b = render("b", Early::Shell);
    complete_all(&b);
    println!("{}: {:?} {}", b.name, b.log.borrow(), b.flowed.get());

    let c = render_shell("c");
    complete_shell(&c);
    complete_all(&c);
    println!("{}: {:?} {}", c.name, c.log.borrow(), c.flowed.get());

    // Its `onAllReady` called before its cell is set: its `unwrap` panics.
    let early = catch_unwind(AssertUnwindSafe(|| render("d", Early::All)));
    println!("d panicked: {}", early.is_err());
}
