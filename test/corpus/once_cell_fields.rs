// A `OnceCell` in a field is the property it holds, absent until it's set
// (ADR 0847): made with `OnceCell::new()` it's left out, `get()` reads it,
// `set(v)` assigns it where it's still empty, and `take()` empties it. Of
// what looks like `None`, `()` or an `Option`, what's set is still there.
use std::cell::OnceCell;
use std::rc::Rc;

#[derive(Debug)]
struct IoInfo {
    name: String,
}

struct Payload {
    status: u32,
    io_info: OnceCell<Rc<IoInfo>>,
}

struct Lazy {
    payload: Rc<Payload>,
    debug: Option<Rc<IoInfo>>,
}

// React's `lazy`: made without its DEV field, which DEV sets.
fn lazy(dev: bool) -> Lazy {
    let payload = Rc::new(Payload {
        status: 0,
        io_info: OnceCell::new(),
    });
    let mut lazy = Lazy {
        payload: Rc::clone(&payload),
        debug: None,
    };
    if dev {
        let io_info = Rc::new(IoInfo {
            name: "lazy".to_string(),
        });
        let _ = payload.io_info.set(Rc::clone(&io_info));
        lazy.debug = Some(io_info);
    }
    lazy
}

fn describe(payload: &Payload) -> String {
    match payload.io_info.get() {
        Some(info) => format!("{} {}", payload.status, info.name),
        None => format!("{} none", payload.status),
    }
}

// Set again where it may be set already: `Err` gives the value back.
fn rename(payload: &Payload, name: &str) -> bool {
    let set = payload.io_info.set(Rc::new(IoInfo {
        name: name.to_string(),
    }));
    match set {
        Ok(()) => true,
        Err(refused) => {
            println!("refused {}", refused.name);
            false
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Flags {
    seen: OnceCell<u8>,
    unit: OnceCell<()>,
    maybe: OnceCell<Option<u8>>,
    label: OnceCell<String>,
}

fn main() {
    let made = lazy(true);
    println!("{}", describe(&made.payload));
    println!("{:?}", made.debug.map(|d| d.name.clone()));
    let plain = lazy(false);
    println!("{}", describe(&plain.payload));
    println!("{} {}", rename(&plain.payload, "late"), describe(&plain.payload));
    println!("{} {}", rename(&made.payload, "again"), describe(&made.payload));
    if let Some(info) = made.payload.io_info.get() {
        println!("got {info:?}");
    }

    let flags = Flags::default();
    println!("{flags:?}");
    println!("{:?} {:?}", flags.unit.set(()), flags.unit.get());
    println!("{:?} {:?}", flags.maybe.set(None), flags.maybe.get());
    println!("{:?}", flags.maybe.set(Some(3)));
    let copy = flags.clone();
    println!("{} {copy:?}", copy == flags);
    println!("{}", flags.label.get_or_init(|| "made".to_string()));
    println!("{}", flags.label.get_or_init(|| "not again".to_string()));
    println!("{} {}", copy == flags, flags == Flags::default());

    let mut owned = Flags {
        seen: OnceCell::new(),
        unit: OnceCell::new(),
        maybe: OnceCell::new(),
        label: OnceCell::new(),
    };
    println!("{:?}", owned.seen.set(1));
    println!("{:?} {:?}", owned.seen.take(), owned.seen.get());
    println!("{:?}", owned.seen.set(2));
    let Flags { seen, label, .. } = &owned;
    println!("{:?} {:?} {:?}", seen.get(), label.set("set".to_string()), label.get());
    println!("{} {:?}", relabel(&owned), owned.label.get());
    *owned.seen.get_mut().unwrap() += 1;
    println!("{:?}", owned.seen.into_inner());
    drops();
}

struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

// What it holds is dropped with the struct, once it's set; a refused `set`
// gives its value back to drop.
struct Holder {
    name: &'static str,
    loud: OnceCell<Loud>,
}

fn drops() {
    let held = Holder {
        name: "held",
        loud: OnceCell::new(),
    };
    let _ = held.loud.set(Loud("first"));
    let again = held.loud.set(Loud("second"));
    println!("{} {}", held.name, again.is_err());
    let empty = Holder {
        name: "empty",
        loud: OnceCell::new(),
    };
    println!("{}", empty.name);
}

// Taken apart where it's given: each a handle on its field.
fn relabel(Flags { seen, label, .. }: &Flags) -> bool {
    println!("{:?}", seen.get());
    label.set("relabelled".to_string()).is_ok()
}
