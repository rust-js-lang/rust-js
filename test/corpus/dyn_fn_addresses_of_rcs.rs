//@ library-refused: `size_of` of a value without one size
// Two `Rc::new`s are two allocations, apart to Rust, so two `Rc<dyn Fn>`s
// made of one function another may hold too are apart where the crate
// reads a `dyn Fn`'s address, `ptr::addr_eq` here: a function
// item's, a `fn` pointer's or a type parameter's a function of its own
// where it's unsized, `(arg0) => f(arg0)`, and a closure copied or cloned
// one where it's given `Rc::new`, so the clones of one `Rc` made before
// it's unsized are still one (#1053).
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

fn double(x: u32) -> u32 {
    x * 2
}

fn words(text: &str) -> usize {
    text.split_whitespace().count()
}

fn shared<F: Fn(u32) -> u32 + 'static>(f: F) -> Rc<dyn Fn(u32) -> u32> {
    Rc::new(f)
}

fn main() {
    // A function item's.
    let first: Rc<dyn Fn(u32) -> u32> = Rc::new(double);
    let second: Rc<dyn Fn(u32) -> u32> = Rc::new(double);
    let again = Rc::clone(&first);
    println!(
        "item {} {} {}",
        std::ptr::addr_eq(&*first, &*second),
        std::ptr::addr_eq(&*first, &*again),
        first(1) + second(2),
    );

    // A cloned closure's, as #1053 found it.
    let count = Rc::new(RefCell::new(4));
    let read = {
        let count = Rc::clone(&count);
        move || *count.borrow()
    };
    let cloned: Rc<dyn Fn() -> u32> = Rc::new(read.clone());
    let moved: Rc<dyn Fn() -> u32> = Rc::new(read);
    *count.borrow_mut() += 1;
    println!("cloned {} {}", std::ptr::addr_eq(&*cloned, &*moved), cloned() + moved());

    // A `Copy` closure's, its `Rc` cloned before it's unsized: one
    // allocation, one function.
    let k = 3;
    let add = move |x: u32| x + k;
    let held = Rc::new(add);
    let one: Rc<dyn Fn(u32) -> u32> = held.clone();
    let other: Rc<dyn Fn(u32) -> u32> = held;
    let apart: Rc<dyn Fn(u32) -> u32> = Rc::new(add);
    println!(
        "copied {} {} {}",
        std::ptr::addr_eq(&*one, &*other),
        std::ptr::addr_eq(&*one, &*apart),
        one(1) + other(2) + apart(3),
    );

    // A type parameter's.
    let a = shared(double);
    let b = shared(add);
    let c = shared(add);
    println!(
        "generic {} {} {}",
        std::ptr::addr_eq(&*a, &*first),
        std::ptr::addr_eq(&*b, &*c),
        a(1) + b(2) + c(3)
    );

    // Of a signature no address is read of, the function itself.
    let measure: Rc<dyn Fn(&str) -> usize> = Rc::new(words);
    println!("measured {}", measure("two words"));

    // An `Arc`'s too.
    let x: Arc<dyn Fn(u32) -> u32 + Send + Sync> = Arc::new(double);
    let y: Arc<dyn Fn(u32) -> u32 + Send + Sync> = Arc::new(double);
    println!("arc {} {}", std::ptr::addr_eq(&*x, &*y), x(1) + y(2));
}
