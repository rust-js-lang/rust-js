//@ run-pass
use std::cell::Cell;
use std::cmp::Ordering;

fn predicates(order: Ordering) -> [bool; 6] {
    [order.is_eq(), order.is_ne(), order.is_lt(), order.is_gt(), order.is_le(), order.is_ge()]
}

fn observed(count: &Cell<u32>, order: Ordering) -> Ordering {
    count.set(count.get() + 1);
    order
}

fn main() {
    let tests: [fn(Ordering) -> bool; 6] = [
        Ordering::is_eq, Ordering::is_ne, Ordering::is_lt,
        Ordering::is_gt, Ordering::is_le, Ordering::is_ge,
    ];
    let count = Cell::new(0);
    for order in [Ordering::Less, Ordering::Equal, Ordering::Greater] {
        println!("{:?}", predicates(order));
        for test in tests {
            println!("{}", test(observed(&count, order)));
        }
        println!("{}", observed(&count, order).is_lt());
    }
    println!("{}", count.get());
    for text in ["a", "z", "é", "😀", "\u{e000}"] {
        let order = text.encode_utf16().cmp("😀".encode_utf16());
        println!("{} {}", order.is_lt(), order.is_gt());
    }
}
