//@ native: wasm32
// What a `RefCell`, a `Cell`, a `Mutex` or an `RwLock` lends is inside it,
// past its borrow flag or its lock's state, where rustc's layout puts it:
// its address is the cell's and that offset, whichever is asked first, the
// cell's own from a reference to it, a field's in the value, the value
// itself, through a guard, `get_mut`, an `Rc` or a struct's field that
// holds a number (#1114). JS keeps the value in the cell's `value`, or is
// the object, a `RefCell` of one (ADR 0863); each was an allocation of its
// own, `4096` where wasm32 gives `4`. Run natively on wasm32, whose `usize`
// is rust-js's: a borrow flag is a `usize`.
use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::{Mutex, RwLock};

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct P {
    a: u8,
    b: u32,
}

struct Counter {
    tag: u8,
    count: RefCell<u32>,
    seen: Cell<u16>,
}

fn at<T>(cell: &T) -> usize {
    cell as *const T as usize
}

fn main() {
    // A `RefCell` of an array, of a number, and of an object, which is it.
    let items = RefCell::new([P { a: 1, b: 2 }, P { a: 3, b: 4 }]);
    let base = &raw const items as usize;
    println!("array {}", &raw const *items.borrow() as usize - base);
    println!("item {}", &raw const (*items.borrow())[1].b as usize - base);
    // Its items' pointers, stepped, moved and compared, each in the cell:
    // a place at an address, one with no place at it, and one found again
    // there, the item's, stepped along its array.
    let (stepped, moved, read, bare) = {
        let held = items.borrow();
        let p = held.as_ptr();
        let second = unsafe { p.add(1) };
        let unplaced = std::ptr::without_provenance::<P>(p.addr() + 8);
        (
            p.wrapping_byte_add(8) == second,
            p.with_addr(p as usize + 8) == second,
            unsafe { (*p.wrapping_byte_add(8)).a },
            unplaced == second,
        )
    };
    println!("stepped {stepped} {moved} {read} {bare}");
    let (first, second) = {
        let held = items.borrow();
        let p = held.as_ptr();
        (p as usize, unsafe { p.add(1) } as usize)
    };
    let again = first as *const P;
    println!("found again {} {} {}", first - base, second - first, unsafe { (*again.add(1)).b });
    let n = RefCell::new(7u32);
    let held = &*n.borrow() as *const u32 as usize;
    println!("number asked first {}", held - &raw const n as usize);
    let mut one = RefCell::new(P { a: 5, b: 6 });
    let lent = {
        let mut guard = one.borrow_mut();
        &raw mut *guard as usize
    };
    let own = &one as *const RefCell<P> as usize;
    println!("object {} {}", lent - own, &raw mut *one.get_mut() as usize - own);
    {
        let p: &P = &one.borrow();
        println!("object field {}", &raw const p.b as usize - own);
    }

    // A `Cell`'s, at its start.
    let mut c = Cell::new(P { a: 7, b: 8 });
    let cb = &raw const c as usize;
    println!("cell {}", &raw mut *c.get_mut() as usize - cb);

    // A lock's, past its state.
    let m = Mutex::new(P { a: 9, b: 10 });
    let guard = m.lock().unwrap();
    println!("mutex {}", &raw const guard.b as usize - &raw const m as usize);
    drop(guard);
    let r = RwLock::new(P { a: 11, b: 12 });
    {
        let read = r.read().unwrap();
        println!("rwlock {}", &raw const *read as usize - &raw const r as usize);
    }
    {
        let mut written = r.write().unwrap();
        println!("rwlock write {}", &raw mut *written as usize - &raw const r as usize);
    }

    // Through an `Rc`, a reference no borrow here made.
    let shared = Rc::new(RefCell::new([P { a: 13, b: 14 }, P { a: 15, b: 16 }]));
    let start = at(&*shared);
    println!("rc {}", &raw const (*shared.borrow())[1].a as usize - start);

    // A struct's `RefCell` field of a number, which holds it, and a `Cell`'s.
    let counter = Counter { tag: 1, count: RefCell::new(3), seen: Cell::new(4) };
    let cs = &raw const counter as usize;
    let count = &raw const counter.count as usize - cs;
    let value = {
        let count = counter.count.borrow();
        &*count as *const u32 as usize - cs
    };
    println!("field {count} {value} {}", &raw const counter.seen as usize - cs);
    *counter.count.borrow_mut() += 1;
    counter.seen.set(counter.seen.get() + 1);
    println!(
        "{} {} {} {:?} {:?} {}",
        counter.tag,
        counter.count.borrow(),
        counter.seen.get(),
        one.borrow(),
        c.get(),
        items.borrow()[0].a + shared.borrow()[0].a + m.lock().unwrap().a + r.read().unwrap().a
    );
}
