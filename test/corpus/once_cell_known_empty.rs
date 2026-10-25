// Where a `OnceCell` is still empty (ADR 0847): one a body made, that
// nothing could have set since, is assigned where it's set; one anything
// else could have set, a call given it, a clone of its `Rc`, a closure or
// a destructor, is set only where it's empty, and its `Err` says so.
use std::cell::OnceCell;
use std::rc::Rc;

struct Node {
    id: u32,
    cache: OnceCell<String>,
}

fn node(id: u32) -> Rc<Node> {
    Rc::new(Node {
        id,
        cache: OnceCell::new(),
    })
}

fn fill(node: &Node) {
    let _ = node.cache.set(format!("filled {}", node.id));
}

fn show(what: &str, node: &Node, set: Result<(), String>) {
    println!("{what}: {:?} {:?}", set, node.cache.get());
}

struct Holder {
    node: Rc<Node>,
}

struct Guard {
    node: Rc<Node>,
}

impl Drop for Guard {
    fn drop(&mut self) {
        let _ = self.node.cache.set("dropped".to_string());
    }
}

fn filled() -> OnceCell<u32> {
    let cell = OnceCell::new();
    let _ = cell.set(1);
    cell
}

fn made(text: &str) -> String {
    println!("making {text}");
    text.to_string()
}

fn main() {
    // Made here and set at once, then again.
    let a = node(1);
    let fresh = Rc::new(Node {
        id: 2,
        cache: OnceCell::new(),
    });
    let set = fresh.cache.set("first".to_string());
    show("fresh", &fresh, set);
    let set = fresh.cache.set("second".to_string());
    show("twice", &fresh, set);

    // Given to a call that sets it.
    let given = Rc::new(Node {
        id: 3,
        cache: OnceCell::new(),
    });
    fill(&given);
    let set = given.cache.set("mine".to_string());
    show("given", &given, set);

    // Through a clone of its `Rc`, held in a struct.
    let shared = Rc::new(Node {
        id: 4,
        cache: OnceCell::new(),
    });
    let holder = Holder {
        node: Rc::clone(&shared),
    };
    fill(&holder.node);
    let set = shared.cache.set("mine".to_string());
    show("held", &shared, set);

    // A clone kept that nothing uses: still empty.
    let kept = Rc::new(Node {
        id: 5,
        cache: OnceCell::new(),
    });
    let other = Holder {
        node: Rc::clone(&kept),
    };
    let set = kept.cache.set("mine".to_string());
    show("kept", &kept, set);
    println!("{}", other.node.id);

    // A holder that runs nothing as it's dropped, gone before it's set.
    let dropped = Rc::new(Node {
        id: 12,
        cache: OnceCell::new(),
    });
    {
        let _gone = Holder {
            node: Rc::clone(&dropped),
        };
    }
    let set = dropped.cache.set("mine".to_string());
    show("dropped", &dropped, set);

    // Set through a clone of its `Rc` that a struct holds: set.
    let through = Rc::new(Node {
        id: 13,
        cache: OnceCell::new(),
    });
    let via = Holder {
        node: Rc::clone(&through),
    };
    let _ = via.node.cache.set("via".to_string());
    let set = through.cache.set("mine".to_string());
    show("through", &through, set);

    // One made inside what's made, an `Rc` in a struct.
    let deep = Holder {
        node: Rc::new(Node {
            id: 14,
            cache: OnceCell::new(),
        }),
    };
    let set = deep.node.cache.set("deep".to_string());
    show("deep", &deep.node, set);

    // Through a raw pointer to a struct that holds it.
    let raw = Rc::new(Node {
        id: 15,
        cache: OnceCell::new(),
    });
    let pointed = Holder {
        node: Rc::clone(&raw),
    };
    let pointer = &raw const pointed;
    // SAFETY: `pointed` lives to the end of `main`.
    fill(unsafe { &(*pointer).node });
    let set = raw.cache.set("mine".to_string());
    show("raw", &raw, set);

    // A closure that sets it, called first.
    let closed = Rc::new(Node {
        id: 6,
        cache: OnceCell::new(),
    });
    let set_it = || fill(&closed);
    set_it();
    let set = closed.cache.set("mine".to_string());
    show("closure", &closed, set);

    // A guard whose drop sets it, dropped where its block ends, or where
    // its loop breaks.
    let guarded = Rc::new(Node {
        id: 7,
        cache: OnceCell::new(),
    });
    {
        let _guard = Guard {
            node: Rc::clone(&guarded),
        };
    }
    let set = guarded.cache.set("mine".to_string());
    show("guarded", &guarded, set);
    let broken = Rc::new(Node {
        id: 8,
        cache: OnceCell::new(),
    });
    loop {
        let _guard = Guard {
            node: Rc::clone(&broken),
        };
        break;
    }
    let set = broken.cache.set("mine".to_string());
    show("broken", &broken, set);

    // Set on one path only, then after both.
    let branched = node(9);
    if a.id == 1 {
        let _ = branched.cache.set("branch".to_string());
    }
    let set = branched.cache.set("after".to_string());
    show("branched", &branched, set);

    // In a loop: one made each turn is empty each turn; one made before it
    // only the first.
    let outer = Rc::new(Node {
        id: 10,
        cache: OnceCell::new(),
    });
    for i in 0..3 {
        let inner = Rc::new(Node {
            id: i,
            cache: OnceCell::new(),
        });
        let _ = inner.cache.set(format!("inner {i}"));
        println!("{:?} {:?}", inner.cache.get(), outer.cache.set(format!("outer {i}")));
    }

    // What it's set to is made first, whether it's set or not.
    let effects = Rc::new(Node {
        id: 11,
        cache: OnceCell::new(),
    });
    let _ = effects.cache.set(made("one"));
    let _ = effects.cache.set(made("two"));
    println!("{:?}", effects.cache.get());

    // A local one, set and set again.
    let local: OnceCell<u32> = OnceCell::new();
    let first = local.set(1);
    let second = local.set(2);
    println!("{first:?} {second:?} {:?}", local.get());
    let given_cell = filled();
    let again = given_cell.set(2);
    println!("{again:?} {:?}", given_cell.get());
    fill(&a);
    println!("{:?}", a.cache.get());
}
