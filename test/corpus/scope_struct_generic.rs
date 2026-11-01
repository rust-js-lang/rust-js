// A generic function's scope struct is its scope too (ADR 1043, amended):
// the struct and its methods are of no type parameter, so each of the
// function's lowerings, its own and a copy its callers need, declares them
// as its own, as ReactFlightReplyClient's processReply takes its
// `resolve`/`reject` callbacks as `impl Fn`.
use std::cell::Cell;
use std::fmt::Display;
use std::rc::Rc;

struct Reply {
    next_id: Cell<u32>,
    on_done: Box<dyn Fn(u32)>,
    parts: std::cell::RefCell<Vec<String>>,
}

impl Reply {
    fn next(&self) -> u32 {
        let id = self.next_id.get();
        self.next_id.set(id + 1);
        id
    }

    fn part(self: &Rc<Self>, text: &str) -> u32 {
        let id = self.next();
        self.parts.borrow_mut().push(format!("{id}:{text}"));
        id
    }

    fn done(&self) {
        (self.on_done)(self.next_id.get());
    }
}

fn process<T: Display>(root: T, on_done: impl Fn(u32) + 'static, on_part: impl Fn(&str)) -> String {
    let reply = Rc::new(Reply {
        next_id: Cell::new(1),
        on_done: Box::new(on_done),
        parts: Default::default(),
    });
    let shown = root.to_string();
    let later = {
        let reply = Rc::clone(&reply);
        move |text: &str| reply.part(text)
    };
    reply.part(&shown);
    later("later");
    for part in reply.parts.borrow().iter() {
        on_part(part);
    }
    reply.done();
    reply.parts.borrow().join(",")
}

struct Counter {
    count: Cell<usize>,
}

impl Counter {
    fn add(&self, n: usize) {
        self.count.set(self.count.get() + n);
    }
}

/// JS calls it, so its own JS takes `T` as JS gives it, and the crate's
/// call of an `Option` calls a copy that boxes its `Some`: each declares
/// the counter's `add`.
pub fn count_some<T: Clone>(items: &[T], keep: impl Fn(&T) -> bool) -> (usize, Option<T>) {
    let counter = Counter { count: Cell::new(0) };
    let mut last = None;
    for item in items {
        if keep(item) {
            counter.add(1);
            last = Some(item.clone());
        }
    }
    (counter.count.get(), last)
}

fn main() {
    let json = process(7, |id| println!("done {id}"), |part| println!("part {part}"));
    println!("{json}");
    println!("{}", process("root", |_| {}, |_| {}));
    println!("{:?}", count_some(&[1, 2, 3, 4], |n| n % 2 == 0));
    println!("{:?}", count_some(&[Some(1), None, Some(3)], |n| n.is_none()));
    println!("{:?}", count_some(&[None::<u8>], |n| n.is_none()));
}
