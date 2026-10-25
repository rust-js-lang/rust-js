// A thread-local `RefCell`'s `set` is its `replace`, the old value dropped
// (issue #249): of one that's the array it holds (ADR 0863), the array
// written in place, so the next read sees it, not `ports.value = [..]`; of
// a `{ value }` box, its value; of what has a destructor, the old dropped
// once the new is in its place, as std's does; of a lazy one set before
// its first use, made of what's given.
use std::cell::RefCell;

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

thread_local! {
    static PORTS: RefCell<Vec<u32>> = const { RefCell::new(Vec::new()) };
    static COUNT: RefCell<u32> = const { RefCell::new(0) };
    static NOISY: RefCell<Vec<Noisy>> = RefCell::new(vec![Noisy("first")]);
    static LATE: RefCell<Vec<u32>> = RefCell::new(first_ports());
}

fn first_ports() -> Vec<u32> {
    println!("made");
    vec![5]
}

fn first_port() -> u32 {
    PORTS.with_borrow(|ports| ports[0])
}

fn main() {
    PORTS.set(vec![1, 0]);
    println!("{}", first_port());
    PORTS.with_borrow_mut(|ports| ports.push(7));
    PORTS.set(vec![3]);
    println!("{:?}", PORTS.with_borrow(|ports| ports.clone()));
    COUNT.set(4);
    COUNT.with_borrow_mut(|count| *count += 1);
    println!("{}", COUNT.with_borrow(|count| *count));
    NOISY.with_borrow(|noisy| println!("{}", noisy.len()));
    NOISY.set(vec![Noisy("second"), Noisy("third")]);
    println!("set");
    NOISY.with_borrow(|noisy| println!("{}", noisy.len()));
    // Set before its first use, made of what's set, never of its
    // initializer (#252), the array it holds.
    LATE.set(vec![8, 9]);
    println!("{:?}", LATE.with_borrow(|late| late.clone()));
    // Emptied, as a thread-local's own destructor at exit isn't run.
    NOISY.set(Vec::new());
    println!("emptied");
}
