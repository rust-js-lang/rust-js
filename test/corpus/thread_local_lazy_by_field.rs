// A thread-local whose initializer reads a field of another's struct, in a
// `with`'s closure, is made as its module loads where nothing after the load
// writes that field, nor the struct whole: React Flight's `const {
// TaintRegistryObjects, .. } = ReactSharedInternals;` (#616, ADR 1006,
// amended). `REGISTRY` reads `registry`, which nothing changes; `DISPATCHER`
// reads `dispatcher`, which `main` sets before its first use, so it's made
// there, as Rust makes it.
// What lends the struct whole may write any field, so what reads one is made
// where it's first used: a function given it, its method, a binding's
// `Object.assign`, a reference copied out of the thread-local. And a field
// that shares what it points to with another, `Shared`'s two `Rc`s of one
// cell, or `Links`' two references a `Cell`'s `get` gives out, is written by
// a write through the other. `Shared` and `Links` are of types of their
// own, so no other thread-local's place may be one with theirs.
use std::cell::Cell;
use std::rc::Rc;

struct Internals {
    dispatcher: Cell<u32>,
    registry: Cell<u32>,
}

impl Internals {
    fn bump_registry(&self) {
        self.registry.set(self.registry.get() + 1);
    }
}

fn bump(internals: &Internals) {
    internals.registry.set(internals.registry.get() + 10);
}

struct Patch {
    registry: u32,
}

#[cfg_attr(rust_js, rust_js::link_name = "Object.assign")]
fn assign(target: &Internals, patch: &Patch) {
    target.registry.set(patch.registry);
}

struct Shared {
    first: Rc<Cell<u16>>,
    second: Rc<Cell<u16>>,
}

fn shared() -> Shared {
    let cell = Rc::new(Cell::new(1));
    Shared {
        first: Rc::clone(&cell),
        second: cell,
    }
}

struct Links {
    first: Cell<Option<&'static Cell<i64>>>,
    second: Cell<Option<&'static Cell<i64>>>,
}

fn links() -> Links {
    let cell: &'static Cell<i64> = Box::leak(Box::new(Cell::new(1)));
    Links {
        first: Cell::new(Some(cell)),
        second: Cell::new(Some(cell)),
    }
}

fn internals() -> Internals {
    Internals {
        dispatcher: Cell::new(1),
        registry: Cell::new(2),
    }
}

thread_local! {
    static INTERNALS: Internals = internals();
    static WHOLE: Internals = internals();
    static METHOD: Internals = internals();
    static ASSIGNED: Internals = internals();
    static NESTED: Internals = internals();
    static HELD: &'static Internals = Box::leak(Box::new(internals()));
    static SHARED: Shared = shared();
    static LINKS: Links = links();
    // Made as its module loads: nothing writes `registry` after.
    static REGISTRY: u32 = INTERNALS.with(|internals| internals.registry.get());
    // Made where they're first used.
    static DISPATCHER: u32 = INTERNALS.with(|internals| internals.dispatcher.get());
    static WHOLE_REGISTRY: u32 = WHOLE.with(|whole| whole.registry.get());
    static WHOLE_DISPATCHER: u32 = WHOLE.with(|whole| whole.dispatcher.get());
    static METHOD_REGISTRY: u32 = METHOD.with(|method| method.registry.get());
    static ASSIGNED_REGISTRY: u32 = ASSIGNED.with(|assigned| assigned.registry.get());
    static NESTED_REGISTRY: u32 = NESTED.with(|nested| nested.registry.get());
    static HELD_REGISTRY: u32 = HELD.with(|&held| held.registry.get());
    static FIRST: u16 = SHARED.with(|shared| shared.first.get());
    static LINKED: i64 = LINKS.with(|links| links.first.get().map_or(0, Cell::get));
}

fn main() {
    INTERNALS.with(|internals| internals.dispatcher.set(5));
    println!("{} {}", REGISTRY.with(|r| *r), DISPATCHER.with(|d| *d));
    WHOLE.with(|whole| bump(whole));
    println!("{} {}", WHOLE_REGISTRY.with(|r| *r), WHOLE_DISPATCHER.with(|d| *d));
    METHOD.with(|method| method.bump_registry());
    println!("{}", METHOD_REGISTRY.with(|r| *r));
    ASSIGNED.with(|assigned| assign(assigned, &Patch { registry: 9 }));
    println!("{}", ASSIGNED_REGISTRY.with(|r| *r));
    NESTED.with(|nested| (0..3).for_each(|_| nested.registry.set(nested.registry.get() + 1)));
    println!("{}", NESTED_REGISTRY.with(|r| *r));
    let held = HELD.with(|held| *held);
    held.registry.set(7);
    println!("{}", HELD_REGISTRY.with(|r| *r));
    SHARED.with(|shared| shared.second.set(4));
    println!("{}", FIRST.with(|f| *f));
    LINKS.with(|links| {
        if let Some(cell) = links.second.get() {
            cell.set(8);
        }
    });
    println!("{}", LINKED.with(|l| *l));
}
