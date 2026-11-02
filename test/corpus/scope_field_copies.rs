// A scope struct's field read through a copy of the program's of its name
// is its variable where nothing can write it first (ADR 1043, amended): React's
// Flight reply client's `let reply_lifetime_controller = new(); self
// .reply_lifetime_controller.set(Some(reply_lifetime_controller));` is
// upstream's `replyLifetimeController = new AbortController();`, read as
// `replyLifetimeController.signal` after, and `if let Some(controller) =
// self.controller.get()` reads the variable, where each was `controller$1`.
// A call between, which may set the field again, or a write of it keeps
// the copy.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Reply {
    controller: Cell<Option<u32>>,
    log: RefCell<Vec<String>>,
}

impl Reply {
    fn attach(self: &Rc<Self>, base: u32) {
        let controller = base * 3;
        self.controller.set(Some(controller));
        let reply = Rc::clone(self);
        let listen = move |options: Options| reply.log.borrow_mut().push(format!("{}", options.signal));
        listen(Options { signal: controller + 1 });
    }

    fn end(&self) {
        if let Some(controller) = self.controller.get() {
            self.log.borrow_mut().push(format!("end {controller}"));
        }
    }
}

struct Options {
    signal: u32,
}

struct Held {
    at: Cell<Option<u32>>,
    log: RefCell<Vec<String>>,
}

impl Held {
    fn bump(&self) {
        self.at.set(Some(99));
    }

    fn stale(&self) {
        if let Some(at) = self.at.get() {
            self.bump();
            self.log.borrow_mut().push(format!("was {at}"));
        }
    }

    fn moved(&self) {
        if let Some(at) = self.at.get() {
            self.at.set(Some(1));
            self.log.borrow_mut().push(format!("moved {at}"));
        }
    }

    fn maybe(&self, again: bool) {
        if let Some(at) = self.at.get() {
            if again {
                self.bump();
            }
            self.log.borrow_mut().push(format!("maybe {at}"));
        }
    }

    fn reattach(&self, base: u32) {
        let at = base + 1;
        self.at.set(Some(at));
        self.bump();
        self.log.borrow_mut().push(format!("set {at}"));
    }
}

fn process(base: u32) -> String {
    let reply = Rc::new(Reply {
        controller: Cell::new(None),
        log: RefCell::new(Vec::new()),
    });
    reply.attach(base);
    reply.end();
    reply.log.borrow().join(",")
}

fn hold(base: u32) -> String {
    let held = Held {
        at: Cell::new(Some(base)),
        log: RefCell::new(Vec::new()),
    };
    held.stale();
    held.moved();
    held.maybe(true);
    held.reattach(base);
    held.stale();
    held.log.borrow().join(",")
}

fn main() {
    println!("{}", process(2));
    println!("{}", hold(5));
}
