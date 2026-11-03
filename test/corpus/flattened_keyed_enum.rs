// A node whose kind, an enum each of whose variants has a field no other
// has, is flattened into it, as alien-signals' nodes are one object each,
// `interface ComputedNode extends ReactiveNode`, told apart by a key,
// `'getter' in node` (#504): each made in place and given as a value,
// matched, lent, its fields changed, and its variant changed at runtime.
// A variant's key is there where it holds `None`, so it's still told.
use std::cell::Cell;

pub enum Kind {
    Signal {
        current: Cell<Option<u32>>,
        pending: Cell<Option<u32>>,
    },
    Computed {
        value: Cell<Option<u32>>,
        #[cfg_attr(rust_js, rust_js::test)]
        getter: fn(Option<u32>) -> u32,
    },
    Effect {
        run: fn() -> u32,
        cleanup: Option<u32>,
    },
    Scope,
}

pub struct Node {
    pub flags: Cell<i32>,
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub kind: Kind,
}

fn node(flags: i32, kind: Kind) -> Node {
    Node {
        flags: Cell::new(flags),
        kind,
    }
}

fn double(previous: Option<u32>) -> u32 {
    previous.map_or(1, |n| n * 2)
}

fn seven() -> u32 {
    7
}

fn name(kind: &Kind) -> &'static str {
    match kind {
        Kind::Signal { .. } => "signal",
        Kind::Computed { .. } => "computed",
        Kind::Effect { .. } => "effect",
        Kind::Scope => "scope",
    }
}

fn read(node: &Node) -> Option<u32> {
    match &node.kind {
        Kind::Signal { current, pending } => {
            current.set(pending.get());
            current.get()
        }
        Kind::Computed { value, getter } => {
            value.set(Some(getter(value.get())));
            value.get()
        }
        Kind::Effect { run, cleanup } => Some(run() + cleanup.unwrap_or(0)),
        Kind::Scope => None,
    }
}

fn is_source(node: &Node) -> bool {
    !matches!(node.kind, Kind::Effect { .. } | Kind::Scope)
}

fn pending(node: &Node) -> Option<u32> {
    let Kind::Signal { pending, .. } = &node.kind else {
        return None;
    };
    pending.get()
}

fn set(node: &Node, value: u32) {
    if let Kind::Signal { pending, .. } = &node.kind {
        pending.set(Some(value));
    }
}

fn become_computed(node: &mut Node) {
    node.kind = Kind::Computed {
        value: Cell::new(None),
        getter: double,
    };
}

// What the new variant holds is read of the old one first.
fn become_cached(node: &mut Node) {
    node.kind = Kind::Computed {
        value: Cell::new(read(node)),
        getter: double,
    };
}

fn become_given(node: &mut Node, kind: Kind) {
    node.kind = kind;
}

fn main() {
    let empty = node(
        1,
        Kind::Signal {
            current: Cell::new(None),
            pending: Cell::new(None),
        },
    );
    let mut nodes = vec![
        empty,
        Node {
            flags: Cell::new(2),
            kind: Kind::Computed {
                value: Cell::new(None),
                getter: double,
            },
        },
        node(
            4,
            Kind::Effect {
                run: seven,
                cleanup: None,
            },
        ),
        node(8, Kind::Scope),
    ];
    set(&nodes[0], 3);
    println!("{}", name(&Kind::Scope));
    for node in &nodes {
        println!(
            "{} {} {:?} {} {:?}",
            name(&node.kind),
            node.flags.get(),
            read(node),
            is_source(node),
            pending(node)
        );
    }
    println!("{:?}", read(&nodes[1]));
    become_computed(&mut nodes[0]);
    become_computed(&mut nodes[3]);
    become_given(
        &mut nodes[1],
        Kind::Effect {
            run: seven,
            cleanup: Some(2),
        },
    );
    become_given(&mut nodes[2], Kind::Scope);
    let mut cached = node(
        16,
        Kind::Signal {
            current: Cell::new(None),
            pending: Cell::new(Some(5)),
        },
    );
    become_cached(&mut cached);
    nodes.push(cached);
    for node in &nodes {
        println!(
            "{} {} {:?} {} {:?}",
            name(&node.kind),
            node.flags.get(),
            read(node),
            is_source(node),
            pending(node)
        );
    }
}
