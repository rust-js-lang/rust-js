// `clone_from` in generic code of a value whose `clone_from` std writes in
// place, a `Vec`'s or a `VecDeque`'s, given by the caller: the caller's
// type's, in std's order of clones and drops, as a monomorphised
// `clone_from` is, never `*dest = source.clone()`'s, every clone then the
// old value dropped (#928). Of a `T` itself, an `Option<T>`, a `Box<T>`
// and items of an `Option<T>`, with a panicking clone too; and of a `T`
// whose `clone_from` is the crate's own, and one a local of the generic
// function holds.
use std::cell::Cell;
use std::collections::VecDeque;
use std::panic::{AssertUnwindSafe, catch_unwind};

thread_local! {
    static BOOM: Cell<u32> = const { Cell::new(0) };
}

/// Prints as it's cloned and dropped, and panics as it's cloned where its
/// number is `BOOM`'s, once.
#[derive(Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl Clone for D {
    fn clone(&self) -> D {
        println!("clone {}", self.0);
        if BOOM.get() == self.0 {
            BOOM.set(0);
            panic!("clone {}", self.0);
        }
        D(self.0)
    }
}

fn run(name: &str, boom: u32, f: impl FnOnce()) {
    println!("-- {name} {boom}");
    BOOM.set(boom);
    let r = catch_unwind(AssertUnwindSafe(f));
    BOOM.set(0);
    println!("{}", if r.is_ok() { "ended" } else { "panicked" });
}

fn assign<T: Clone>(dest: &mut T, source: &T) {
    dest.clone_from(source);
}

fn assign_option<T: Clone>(dest: &mut Option<T>, source: &Option<T>) {
    dest.clone_from(source);
}

fn assign_box<T: Clone>(dest: &mut Box<T>, source: &Box<T>) {
    dest.clone_from(source);
}

fn assign_items<T: Clone>(dest: &mut [Option<T>], source: &[Option<T>]) {
    dest.clone_from_slice(source);
}

fn assign_local<T: Clone + std::fmt::Debug>(start: T, source: &T) {
    let mut local = start;
    local.clone_from(source);
    println!("{local:?}");
}

/// Its own `clone_from`, which std's `Option`'s and a generic caller's
/// call.
#[derive(Debug)]
struct Counted(u32);

impl Clone for Counted {
    fn clone(&self) -> Counted {
        println!("counted clone {}", self.0);
        Counted(self.0)
    }

    fn clone_from(&mut self, source: &Counted) {
        println!("counted clone_from {} <- {}", self.0, source.0);
        self.0 = source.0;
    }
}

fn main() {
    for boom in [0, 1, 3] {
        run("T", boom, || {
            let mut w = vec![D(7)];
            assign(&mut w, &vec![D(3), D(1)]);
            println!("{w:?}");
        });
        run("T of a VecDeque", boom, || {
            let mut w: VecDeque<D> = vec![D(7), D(8)].into();
            assign(&mut w, &vec![D(3), D(1)].into());
            println!("{w:?}");
        });
        run("Option<T>", boom, || {
            let mut w = Some(vec![D(7)]);
            assign_option(&mut w, &Some(vec![D(3), D(1)]));
            println!("{w:?}");
            let mut none = None;
            assign_option(&mut none, &Some(vec![D(5)]));
            println!("{none:?}");
            let mut some = Some(vec![D(6)]);
            assign_option(&mut some, &None);
            println!("{some:?}");
        });
        run("Option<T> of a VecDeque", boom, || {
            let mut w: Option<VecDeque<D>> = Some(vec![D(7), D(8)].into());
            assign_option(&mut w, &Some(vec![D(3), D(1)].into()));
            println!("{w:?}");
        });
        run("Box<T>", boom, || {
            let mut w = Box::new(vec![D(7)]);
            assign_box(&mut w, &Box::new(vec![D(3), D(1)]));
            println!("{w:?}");
        });
        run("items of Option<T>", boom, || {
            let mut w = vec![Some(vec![D(7)]), None, Some(vec![D(8), D(9)])];
            assign_items(&mut w, &[Some(vec![D(3), D(1)]), Some(vec![D(4)]), None]);
            println!("{w:?}");
        });
        run("a local of T", boom, || {
            assign_local(vec![D(7), D(8)], &vec![D(3), D(1)])
        });
    }
    let mut c = Counted(1);
    assign(&mut c, &Counted(2));
    let mut o = Some(Counted(3));
    assign_option(&mut o, &Some(Counted(4)));
    let mut b = Box::new(Counted(5));
    assign_box(&mut b, &Box::new(Counted(6)));
    let mut items = [Some(Counted(7))];
    assign_items(&mut items, &[Some(Counted(8))]);
    assign_local(Counted(9), &Counted(10));
    // A `None` written over: what the dictionary writes, given back.
    assign_local(None, &Some(vec![D(11)]));
    println!("{c:?} {o:?} {b:?} {items:?}");
    let mut n = Some(1);
    assign_option(&mut n, &Some(2));
    // Of a `T` whose `clone_from` is std's provided: a clone written over.
    let mut numbers = [Some(3), None];
    assign_items(&mut numbers, &[Some(4), Some(5)]);
    println!("{n:?} {numbers:?}");
}
