//@ library-differs
// Two boxes are two allocations, apart to Rust, so two `Box<dyn Fn>`s
// made of one function another may hold too, a `fn` pointer, a closure
// copied or cloned, or a type parameter's, are apart where the crate reads
// a `dyn Fn`'s address: each a function of its own that calls it,
// `(arg0) => f(arg0)`, where it's unsized. One made of a closure written in
// its `Box::new` is that closure (#1053).
struct Loud(&'static str);

impl Clone for Loud {
    fn clone(&self) -> Self {
        println!("clone {}", self.0);
        Loud(self.0)
    }
}

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn double(x: u32) -> u32 {
    x * 2
}

fn triple(x: u32) -> u32 {
    x * 3
}

fn boxed(f: fn(u32) -> u32) -> Box<dyn Fn(u32) -> u32> {
    Box::new(f)
}

fn boxed_generic<F: Fn(u32) -> u32 + 'static>(f: F) -> Box<dyn Fn(u32) -> u32> {
    Box::new(f)
}

fn main() {
    // A `fn` pointer's.
    let a = boxed(double);
    let b = boxed(double);
    println!("pointer {} {}", std::ptr::addr_eq(&*a, &*b), a(1) + b(2));

    // A `Copy` closure's, and a type parameter's.
    let k = 3;
    let add = move |x: u32| x + k;
    let first: Box<dyn Fn(u32) -> u32> = Box::new(add);
    let second: Box<dyn Fn(u32) -> u32> = Box::new(add);
    let third = boxed_generic(add);
    let fourth = boxed_generic(triple as fn(u32) -> u32);
    let fifth = boxed_generic(triple as fn(u32) -> u32);
    println!(
        "copied {} {} {} {}",
        std::ptr::addr_eq(&*first, &*second),
        std::ptr::addr_eq(&*second, &*third),
        std::ptr::addr_eq(&*fourth, &*fifth),
        first(1) + second(2) + third(3) + fourth(4) + fifth(5),
    );

    // A cloned closure's that drops what it holds, each box its clone's.
    let loud = Loud("held");
    let read = move || {
        let held = &loud;
        held.0.len()
    };
    {
        let cloned: Box<dyn Fn() -> usize> = Box::new(read.clone());
        let moved: Box<dyn Fn() -> usize> = Box::new(read);
        println!("cloned {} {}", std::ptr::addr_eq(&*cloned, &*moved), cloned() + moved());
        drop(cloned);
        println!("one gone");
    }

    // Each of a table's `fn` pointers boxed: only the first box is itself.
    let table: Vec<fn(u32) -> u32> = vec![double, triple, double];
    let boxes: Vec<Box<dyn Fn(u32) -> u32>> = table
        .iter()
        .map(|&f| Box::new(f) as Box<dyn Fn(u32) -> u32>)
        .collect();
    let head = &*boxes[0];
    let same = boxes.iter().filter(|b| std::ptr::addr_eq(&***b, head)).count();
    println!("table {same} {}", boxes.iter().map(|b| b(1)).sum::<u32>());

    // A closure written in its `Box::new` is its own already.
    let made: Box<dyn Fn(u32) -> u32> = Box::new(move |x| x + k);
    println!("made {}", std::ptr::addr_eq(&*made, &*first));
}
