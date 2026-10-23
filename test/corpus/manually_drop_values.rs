// A `ManuallyDrop` is what it holds (ADR 0584): its `==`, its order and its
// clone are its value's, as std's are.
use std::mem::ManuallyDrop;
#[derive(Debug, Clone, PartialEq, PartialOrd)]
struct P {
    x: i32,
    y: i32,
}
fn main() {
    let a = ManuallyDrop::new(P { x: 1, y: 2 });
    let b = ManuallyDrop::new(P { x: 1, y: 3 });
    println!("{} {} {:?} {:?}", a == b, a != a.clone(), a.partial_cmp(&b), *a);
    let v = vec![ManuallyDrop::new(P { x: 2, y: 1 }), ManuallyDrop::new(P { x: 1, y: 1 })];
    println!(
        "{:?} {}",
        v.iter().map(|p| p.x).collect::<Vec<_>>(),
        v.contains(&ManuallyDrop::new(P { x: 1, y: 1 }))
    );
    let n = ManuallyDrop::new(5u8);
    println!("{} {}", *n + 1, n == ManuallyDrop::new(5));
}
