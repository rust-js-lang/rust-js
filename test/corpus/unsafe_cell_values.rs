// An `UnsafeCell<T>` is `{ value }`, as a `Cell` is (#1420): `new(v)` makes
// one, `get()` is the cell itself, the handle a raw pointer to its value
// is (ADR 0407), written and read through, `get_mut()` its value's place
// and `into_inner()` its value. Held in a local, a struct's field, an
// `Rc` shared by two owners, rand 0.7's `ThreadRng`, and generic code's;
// what it holds dropped as it drops or is written over, its `Default`
// and its `Debug`, which shows nothing of what it holds.
use std::cell::UnsafeCell;
use std::rc::Rc;

struct Rng {
    state: u32,
}

impl Rng {
    fn next(&mut self) -> u32 {
        self.state = self.state.wrapping_mul(1103515245).wrapping_add(12345);
        self.state >> 16
    }
}

struct Thread {
    rng: Rc<UnsafeCell<Rng>>,
}

impl Thread {
    fn next(&mut self) -> u32 {
        let rng = unsafe { &mut *self.rng.get() };
        rng.next()
    }
}

struct Counter {
    hits: UnsafeCell<u32>,
    log: UnsafeCell<Vec<String>>,
}

impl Counter {
    fn hit(&self, what: &str) -> u32 {
        unsafe {
            *self.hits.get() += 1;
            (*self.log.get()).push(what.to_string());
            *self.hits.get()
        }
    }
}

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Default)]
struct Slots {
    n: UnsafeCell<u32>,
    names: UnsafeCell<Vec<String>>,
}

fn bump(cell: &mut UnsafeCell<u32>) {
    *cell.get_mut() += 10;
}

fn swap_in<T>(cell: &UnsafeCell<T>, value: T) -> T {
    unsafe { std::mem::replace(&mut *cell.get(), value) }
}

fn main() {
    let c = UnsafeCell::new(5u32);
    unsafe {
        *c.get() += 1;
    }
    println!("{}", c.into_inner());
    let mut text = UnsafeCell::new(String::from("a"));
    text.get_mut().push('b');
    println!("{}", text.get_mut());
    let shared = Rc::new(UnsafeCell::new(Rng { state: 1 }));
    let mut a = Thread { rng: shared.clone() };
    let mut b = Thread { rng: shared };
    println!("{} {} {}", a.next(), b.next(), a.next());
    let counter = Counter { hits: UnsafeCell::new(0), log: UnsafeCell::new(Vec::new()) };
    println!("{} {}", counter.hit("x"), counter.hit("y"));
    let Counter { hits, log } = counter;
    println!("{} {:?}", hits.into_inner(), log.into_inner());
    let held = UnsafeCell::new(vec![1, 2]);
    let old = swap_in(&held, vec![3]);
    println!("{old:?} {:?}", held.into_inner());
    let number = UnsafeCell::new(7i64);
    println!("{} {}", swap_in(&number, 8), number.into_inner());
    {
        let _held = UnsafeCell::new(D(1));
        println!("held");
    }
    let cell = UnsafeCell::new(D(2));
    unsafe {
        *cell.get() = D(3);
    }
    println!("written over");
    drop(cell);
    let slots = Slots::default();
    unsafe {
        *slots.n.get() = 4;
    }
    println!("{:?} {}", slots.n, slots.names.into_inner().len());
    let mut bumped = UnsafeCell::new(1);
    bump(&mut bumped);
    let mut many = vec![UnsafeCell::new(2), UnsafeCell::new(3)];
    bump(&mut many[1]);
    println!("{} {}", bumped.into_inner(), unsafe { *many[1].get() });
    let zero: UnsafeCell<u8> = UnsafeCell::default();
    println!("{}", zero.into_inner());
}
