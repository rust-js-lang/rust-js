//@ library-differs
// A generic graph's node holds its caller's kind, a type parameter
// flattened into it, as alien-signals' system.ts knows only the
// `ReactiveNode` each of index.ts's nodes extends (#504): the graph makes
// and links nodes of any kind, and the caller's enum, told by its keys,
// is each node's own, `'getter' in node`, matched where it's given. A
// struct given for the kind is flattened as any struct is (ADR 0204), as
// React's `DetailedHTMLProps<AnchorHTMLAttributes<'_>, ..>` is.
use std::cell::{Cell, RefCell};
use std::rc::Rc;

mod graph {
    use std::cell::{Cell, RefCell};
    use std::rc::Rc;

    pub struct Node<K> {
        pub flags: Cell<i32>,
        pub subs: RefCell<Vec<Rc<Node<K>>>>,
        #[cfg_attr(rust_js, rust_js::flatten)]
        pub kind: K,
    }

    pub fn node<K>(flags: i32, kind: K) -> Rc<Node<K>> {
        Rc::new(Node {
            flags: Cell::new(flags),
            subs: RefCell::new(Vec::new()),
            kind,
        })
    }

    pub fn link<K>(dep: &Rc<Node<K>>, sub: &Rc<Node<K>>) {
        dep.subs.borrow_mut().push(Rc::clone(sub));
        sub.flags.set(sub.flags.get() | 16);
    }

    pub fn each_sub<K>(dep: &Node<K>, mut f: impl FnMut(&Node<K>)) {
        for sub in dep.subs.borrow().iter() {
            f(sub);
        }
    }
}

enum Kind {
    Signal {
        value: Cell<Option<i32>>,
    },
    Computed {
        #[cfg_attr(rust_js, rust_js::test)]
        getter: fn(i32) -> i32,
        cached: Cell<Option<i32>>,
    },
    Scope,
}

type Node = Rc<graph::Node<Kind>>;

fn signal(value: Option<i32>) -> Node {
    graph::node(
        1,
        Kind::Signal {
            value: Cell::new(value),
        },
    )
}

fn computed(getter: fn(i32) -> i32) -> Node {
    Rc::new(graph::Node {
        kind: Kind::Computed {
            getter,
            cached: Cell::new(None),
        },
        flags: Cell::new(2),
        subs: RefCell::new(Vec::new()),
    })
}

fn update(node: &graph::Node<Kind>, input: i32) -> String {
    match &node.kind {
        Kind::Signal { value } => {
            value.set(Some(input));
            format!("signal {:?}", value.get())
        }
        Kind::Computed { getter, cached } => {
            cached.set(Some(getter(input)));
            format!("computed {:?}", cached.get())
        }
        Kind::Scope => "scope".to_string(),
    }
}

struct Label {
    text: String,
    width: Cell<u32>,
}

fn label(flags: i32, text: &str, width: u32) -> Rc<graph::Node<Label>> {
    graph::node(
        flags,
        Label {
            text: text.to_string(),
            width: Cell::new(width),
        },
    )
}

fn triple(n: i32) -> i32 {
    n * 3
}

fn main() {
    let source = signal(None);
    let derived = computed(triple);
    let scope = graph::node(4, Kind::Scope);
    graph::link(&source, &derived);
    graph::link(&source, &scope);
    graph::link(&derived, &scope);
    println!("{}", update(&source, 5));
    graph::each_sub(&source, |sub| {
        println!("{} {}", sub.flags.get(), update(sub, 2));
    });
    graph::each_sub(&derived, |sub| {
        println!("{} {}", sub.flags.get(), update(sub, 1));
    });
    println!("{}", update(&derived, 7));
    let total = label(8, "total", 5);
    graph::link(&total, &label(1, "sub", 3));
    graph::each_sub(&total, |sub| {
        sub.kind.width.set(sub.kind.width.get() + total.kind.width.get());
        println!("{} {} {}", sub.flags.get(), sub.kind.text, sub.kind.width.get());
    });
    println!("{} {}", total.kind.text, total.flags.get());
}
