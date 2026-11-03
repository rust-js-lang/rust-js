// Callbacks made before the request they need, read through a cell set once
// it's made, as React's Fizz server's entries do. Nothing `create_request`
// runs before it returns may call them: no body it may run names a field
// they're kept in, though it lends the request to its helpers, gives std a
// closure and a task of its own to hash, and reads a thread-local. So each
// runs only once the cell is set, and its `unwrap` of the upgrade is the
// request itself, upstream's `request` (ADR 0317, amended, rd621). Where
// what it runs may call one, its `unwrap` stays: a request that completes
// its shell before it returns (`owner_handle`), one that calls a `dyn Fn`
// it's given, which may be anything, one that drops what holds a
// destructor that calls one, and one that calls a `dyn` of the crate's
// trait, one of whose impls may run a body rust-js doesn't read.

use std::cell::{Cell, OnceCell, RefCell};
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::rc::{Rc, Weak};

type Callback = dyn Fn();

struct Task {
    id: u32,
}

impl PartialEq for Task {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self, other)
    }
}

impl Eq for Task {}

impl Hash for Task {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::ptr::hash(self, state);
    }
}

struct Request {
    name: String,
    flowed: Cell<u32>,
    log: RefCell<Vec<String>>,
    tasks: RefCell<HashSet<Rc<Task>>>,
    on_all_ready: Rc<Callback>,
    on_shell_ready: Rc<Callback>,
}

thread_local! {
    static NEXT_ID: Cell<u32> = const { Cell::new(1) };
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
        tasks: RefCell::new(HashSet::new()),
        on_all_ready: on_all_ready.unwrap_or_else(|| Rc::new(noop)),
        on_shell_ready: on_shell_ready.unwrap_or_else(|| Rc::new(noop)),
    })
}

fn create_task(request: &Rc<Request>) -> Rc<Task> {
    let id = NEXT_ID.with(|next| next.replace(next.get() + 1));
    let task = Rc::new(Task { id });
    request.tasks.borrow_mut().insert(Rc::clone(&task));
    task
}

fn create_request(name: &str, on_all_ready: Option<Rc<Callback>>, on_shell_ready: Option<Rc<Callback>>) -> Rc<Request> {
    let request = request_instance(name, on_all_ready, on_shell_ready);
    let task = create_task(&request);
    let names: Vec<String> = ["root", "shell"].iter().map(|part| format!("{part} {}", task.id)).collect();
    request.log.borrow_mut().extend(names);
    request
}

// One that calls what it's given, which may be anything.
fn create_logged_request(name: &str, on_all_ready: Option<Rc<Callback>>, logged: &dyn Fn(&str)) -> Rc<Request> {
    let request = request_instance(name, on_all_ready, None);
    logged(&request.name);
    request
}

// A closer whose drop calls what it holds, which may be anything, in a
// field of what a request's maker drops.
struct Closer {
    name: &'static str,
    close: Rc<Callback>,
}

impl Drop for Closer {
    fn drop(&mut self) {
        let close = &self.close;
        close();
    }
}

struct Held {
    closer: Closer,
}

fn held(name: &'static str) -> Held {
    Held {
        closer: Closer {
            name,
            close: Rc::new(|| println!("closed")),
        },
    }
}

// One that drops what it's given, which holds a closer: only its field
// says it has a destructor.
fn create_closing_request(name: &str, on_all_ready: Option<Rc<Callback>>, held: Held) -> Rc<Request> {
    let request = request_instance(name, on_all_ready, None);
    request.log.borrow_mut().push(format!("held {}", held.closer.name));
    request
}

// A matcher of the crate's, which a `dyn` may be any of: one compares by a
// derived `PartialEq`, whose body rust-js doesn't read.
trait Matcher {
    fn matches(&self) -> bool;
}

#[derive(PartialEq)]
enum Kind {
    Root,
    Leaf,
}

struct Exact<T>(T, T);

impl<T: PartialEq> Matcher for Exact<T> {
    fn matches(&self) -> bool {
        self.0 == self.1
    }
}

fn create_matched_request(name: &str, on_all_ready: Option<Rc<Callback>>, matcher: &dyn Matcher) -> Rc<Request> {
    let request = request_instance(name, on_all_ready, None);
    request.log.borrow_mut().push(format!("matched {}", matcher.matches()));
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

fn render_logged(name: &str) -> Rc<Request> {
    let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
    let on_all_ready: Rc<Callback> = {
        let cell = Rc::clone(&request_cell);
        Rc::new(move || {
            let request = cell.get().and_then(Weak::upgrade).unwrap();
            start_flowing(&request, "logged");
        })
    };
    let request = create_logged_request(name, Some(on_all_ready), &|name| println!("made {name}"));
    let _ = request_cell.set(Rc::downgrade(&request));
    request
}

fn render_closing(name: &str) -> Rc<Request> {
    let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
    let on_all_ready: Rc<Callback> = {
        let cell = Rc::clone(&request_cell);
        Rc::new(move || {
            let request = cell.get().and_then(Weak::upgrade).unwrap();
            start_flowing(&request, "closing");
        })
    };
    let request = create_closing_request(name, Some(on_all_ready), held("closer"));
    let _ = request_cell.set(Rc::downgrade(&request));
    request
}

fn render_matched(name: &str, kind: Kind) -> Rc<Request> {
    let request_cell: Rc<OnceCell<Weak<Request>>> = Rc::new(OnceCell::new());
    let on_all_ready: Rc<Callback> = {
        let cell = Rc::clone(&request_cell);
        Rc::new(move || {
            let request = cell.get().and_then(Weak::upgrade).unwrap();
            start_flowing(&request, "matched");
        })
    };
    let request = create_matched_request(name, Some(on_all_ready), &Exact(kind, Kind::Leaf));
    let _ = request_cell.set(Rc::downgrade(&request));
    request
}

fn main() {
    let a = render("a");
    complete_shell(&a);
    complete_all(&a);
    complete_all(&a);
    println!("{}: {:?} {} {}", a.name, a.log.borrow(), a.flowed.get(), a.tasks.borrow().len());

    let b = render_logged("b");
    complete_all(&b);
    println!("{}: {:?} {}", b.name, b.log.borrow(), b.flowed.get());

    let d = render_closing("d");
    complete_all(&d);
    println!("{}: {:?} {}", d.name, d.log.borrow(), d.flowed.get());

    for (name, kind) in [("e", Kind::Root), ("f", Kind::Leaf)] {
        let request = render_matched(name, kind);
        complete_all(&request);
        println!("{}: {:?} {}", request.name, request.log.borrow(), request.flowed.get());
    }
}
