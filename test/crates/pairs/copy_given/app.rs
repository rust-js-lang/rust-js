//! A crate giving a library's generic functions a `Point` it changes in
//! place, which they copy, and numbers, which are their own copies.
#[derive(Clone, Copy, Debug)]
struct Point {
    x: i32,
}

pub fn main() {
    let context = dep::create_context(Point { x: 1 });
    let mut taken = context.current_value.replace(Point { x: 2 });
    taken.x += 10;
    println!("{:?} {:?} {:?}", taken, context.current_value.get(), context.current_value2.get());
    let (mut a, b) = dep::twice(Point { x: 3 });
    a.x = 4;
    println!("{a:?} {b:?}");
    let points = [Point { x: 5 }];
    let mut found = dep::first(&points).unwrap();
    found.x = 6;
    println!("{found:?} {:?}", points[0]);
    let mut current = dep::current_or(None, Point { x: 7 });
    current.x += 1;
    println!("{current:?}");
    let numbers = dep::create_context(7);
    println!("{} {:?} {:?}", numbers.current_value2.get(), dep::twice(8), dep::first(&[9]));
}
