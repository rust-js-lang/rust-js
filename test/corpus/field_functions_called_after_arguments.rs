// A function a field holds, called with what it's given needing
// statements: a field Rust can't change while the body runs is the method
// call's, `hooks.f(..)`, read after what's given; one Rust can change
// between, of a `&mut` or a cell, is read where Rust reads it (rd285).
use std::cell::Cell;

struct Hooks {
    f: fn(u32) -> u32,
}

struct Outer {
    inner: Hooks,
}

struct Celled {
    f: Cell<fn(u32) -> u32>,
}

static HOOKS: Hooks = Hooks { f: double };

fn hooks() -> &'static Hooks {
    &HOOKS
}

fn double(n: u32) -> u32 {
    n * 2
}

fn triple(n: u32) -> u32 {
    n * 3
}

fn given(hooks: &Hooks, n: u32) -> u32 {
    (hooks.f)(if n > 3 {
        println!("big {n}");
        n
    } else {
        n + 1
    })
}

fn global(n: u32) -> u32 {
    (hooks().f)(match n {
        0 => {
            println!("zero");
            1
        }
        _ => n,
    })
}

fn swapped(n: u32) -> u32 {
    let mut outer = Outer { inner: Hooks { f: double } };
    let inner = &mut outer.inner;
    (inner.f)({
        inner.f = triple;
        n
    })
}

fn rebound<'a>(mut hooks: &'a Hooks, other: &'a Hooks, n: u32) -> u32 {
    (hooks.f)({
        hooks = other;
        n
    })
}

impl Outer {
    fn inner(&self) -> &Hooks {
        &self.inner
    }
}

fn borrowed(n: u32) -> u32 {
    let mut outer = Outer { inner: Hooks { f: double } };
    (outer.inner().f)({
        outer.inner.f = triple;
        n
    })
}

fn celled(celled: &Celled, n: u32) -> u32 {
    (celled.f.get())({
        celled.f.set(triple);
        n
    })
}

fn main() {
    let hooks = Hooks { f: triple };
    println!("{} {}", given(&hooks, 2), given(&hooks, 5));
    println!("{} {}", global(0), global(4));
    println!("{} {}", swapped(5), borrowed(5));
    println!("{}", rebound(&hooks, &HOOKS, 5));
    let cell = Celled { f: Cell::new(double) };
    println!("{} {}", celled(&cell, 5), celled(&cell, 5));
}
