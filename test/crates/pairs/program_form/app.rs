//! A crate giving a library's generic functions a `Point` it changes in
//! place, which they must copy, and a value with a destructor, which a
//! panic must drop; and numbers, which they needn't do either for.
use std::panic::{self, AssertUnwindSafe};

#[derive(Clone, Copy, Debug)]
struct Point {
    x: i32,
}

struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

pub fn main() {
    let context = dep::create_context(Point { x: 1 });
    let mut taken = context.current_value.replace(Point { x: 2 });
    taken.x += 10;
    println!("{:?} {:?} {:?}", taken, context.current_value.get(), context.current_value2.get());
    let (state, dispatcher) = dep::use_state(Loud(1), Some("client"));
    println!("{} {dispatcher}", state.0);
    let outside = panic::catch_unwind(AssertUnwindSafe(|| dep::use_state(Loud(2), None).1));
    println!("outside a render: {}", outside.is_err());
    let asked = panic::catch_unwind(AssertUnwindSafe(|| dep::dispatched(Loud(5), None)));
    println!("asked outside a render: {}", asked.is_err());
    println!("{}", dep::dispatched(Loud(6), Some("worker")));
    let (a, b) = dep::pair_of(7, Loud(4));
    println!("{a} {b}");
    let (mut c, d) = dep::twice(Point { x: 3 });
    c.x = 4;
    println!("{c:?} {d:?} {}", dep::marks());
    let numbers = dep::create_context(7);
    println!("{} {}", numbers.current_value2.get(), dep::use_state(3, Some("server")).0);
}
