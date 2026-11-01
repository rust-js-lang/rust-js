// A private struct whose one value a function makes, and only its methods
// use, is that function's scope: its methods are functions declared in the
// function, closing over the value, as ReactFlightReplyClient's
// `processReply` writes its `resolveToJSON` and `serializeModel`. Each call
// makes a scope of its own.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

struct Reply {
    prefix: String,
    next_part_id: Cell<u32>,
    pending: Cell<u32>,
    parts: RefCell<Vec<String>>,
    settled: Cell<bool>,
    limit: u32,
}

impl Reply {
    fn resolve(&self, value: &str) -> String {
        self.settled.set(true);
        format!("{}{value}", self.prefix)
    }

    fn resolve_to_json(self: &Rc<Self>, key: &str, depth: u32) -> String {
        if depth > 0 && depth <= self.limit {
            let id = self.next_part_id.get();
            self.next_part_id.set(id + 1);
            let inner = self.serialize_model(key, depth - 1);
            self.parts.borrow_mut().push(inner);
            return format!("${id}");
        }
        key.to_uppercase()
    }

    fn serialize_model(self: &Rc<Self>, key: &str, depth: u32) -> String {
        self.pending.set(self.pending.get() + 1);
        let reply = Rc::clone(self);
        let later = move |k: &str| format!("{}{}", reply.resolve_to_json(k, 0), reply.pending.get());
        let json = self.resolve_to_json(key, depth);
        format!("{json}/{}", later("x"))
    }
}

fn process_reply(root: &str, prefix: &str, depth: u32) -> String {
    let reply = Rc::new(Reply {
        prefix: prefix.to_owned(),
        next_part_id: Cell::new(1),
        pending: Cell::new(0),
        parts: RefCell::new(Vec::new()),
        settled: Cell::new(false),
        limit: depth,
    });
    let json = reply.serialize_model(root, depth);
    let parts = reply.parts.borrow().join(",");
    let settled = reply.settled.get();
    let resolve = reply.resolve(&json);
    format!("{resolve} [{parts}] {} {settled} {}", reply.pending.get(), reply.settled.get())
}

struct Counter {
    count: u32,
    step: u32,
    times: u32,
}

impl Counter {
    /// Adds its step.
    fn tick(&mut self) {
        self.count += self.step;
    }

    /// Ticks `n` times.
    fn ticks(&mut self, n: u32) -> u32 {
        let counter = n;
        for _ in 0..counter {
            self.tick();
        }
        self.count + self.times
    }
}

fn count_up(mut step: u32, n: u32) -> u32 {
    let mut counter = Counter { count: n, step, times: n };
    let first = counter.ticks(n - 1);
    step += 1;
    counter.tick();
    first * 100 + counter.count + step + n
}

fn main() {
    println!("{}", process_reply("a", "p:", 3));
    println!("{}", process_reply("b", "q:", 1));
    println!("{} {}", count_up(2, 3), count_up(5, 1));
}
