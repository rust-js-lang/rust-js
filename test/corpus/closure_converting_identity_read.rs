//@ library-differs
// A closure that only converts what it's given and passes it on to a
// function value it captured is that function only where the crate reads
// no function's identity (ADR 0605, amended; ADR 1043's #500 rule): two
// boxes of closures of one function that `ptr::addr_eq` compares are two
// in Rust, so each keeps its arrow, and the comparison is `false`. The
// function is a `fn` pointer, which each closure holds, so each box has
// an address of its own.
use std::rc::Rc;

struct Node {
    compute: Box<dyn Fn(Option<Rc<u32>>) -> Rc<u32>>,
}

fn computed(getter: impl Fn(Option<&u32>) -> u32 + 'static) -> Node {
    Node {
        compute: Box::new(move |previous: Option<Rc<u32>>| Rc::new(getter(previous.as_deref()))),
    }
}

fn double(previous: Option<&u32>) -> u32 {
    previous.map_or(1, |n| n * 2)
}

fn main() {
    let getter: fn(Option<&u32>) -> u32 = double;
    let a = computed(getter);
    let b = computed(getter);
    let same = std::ptr::addr_eq(&*a.compute, &*b.compute);
    println!("same {same}");
    let first = (a.compute)(None);
    println!("{first} {}", (b.compute)(Some(first.clone())));
}
