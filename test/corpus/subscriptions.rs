// A program's RAII: a subscription whose drop removes its handler from
// its bus, held in a component's `Vec`, a bus an `Rc<RefCell<..>>` it
// reaches by a `Weak`: each drops where Rust's does, a scope's end,
// `retain`, the program's (ADRs 0098, 0320).
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

type Handler = Box<dyn Fn(&str)>;

struct Bus {
    next: u32,
    handlers: HashMap<u32, Handler>,
}

struct Subscription {
    id: u32,
    bus: Weak<RefCell<Bus>>,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        if let Some(bus) = self.bus.upgrade() {
            bus.borrow_mut().handlers.remove(&self.id);
            println!("unsubscribed {}", self.id);
        }
    }
}

fn subscribe(bus: &Rc<RefCell<Bus>>, f: impl Fn(&str) + 'static) -> Subscription {
    let mut b = bus.borrow_mut();
    let id = b.next;
    b.next += 1;
    b.handlers.insert(id, Box::new(f));
    Subscription {
        id,
        bus: Rc::downgrade(bus),
    }
}

fn publish(bus: &Rc<RefCell<Bus>>, msg: &str) {
    let b = bus.borrow();
    let mut ids: Vec<&u32> = b.handlers.keys().collect();
    ids.sort();
    for id in ids {
        (b.handlers[id])(msg);
    }
}

struct Component {
    name: String,
    subs: Vec<Subscription>,
}

impl Component {
    fn new(name: &str, bus: &Rc<RefCell<Bus>>) -> Component {
        let n = name.to_string();
        let first = subscribe(bus, move |m| println!("{n} got {m}"));
        let count = Rc::new(RefCell::new(0));
        let c = Rc::clone(&count);
        let second = subscribe(bus, move |_| *c.borrow_mut() += 1);
        Component {
            name: name.to_string(),
            subs: vec![first, second],
        }
    }
}

fn main() {
    let bus = Rc::new(RefCell::new(Bus {
        next: 1,
        handlers: HashMap::new(),
    }));
    let a = Component::new("a", &bus);
    {
        let b = Component::new("b", &bus);
        publish(&bus, "hello");
        println!("{} has {}", b.name, b.subs.len());
    }
    publish(&bus, "again");
    let mut comps = vec![a];
    comps.retain(|c| c.name != "a");
    publish(&bus, "empty");
    println!("left {}", bus.borrow().handlers.len());
}
