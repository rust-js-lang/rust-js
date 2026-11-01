//! A crate calling its dependency named `core` (#455), whose items are
//! that crate's, and std's alike, each what it does (ADR 1052).
pub fn main() {
    println!("{}", core::double(21));
    println!("{}", core::Point { x: 1, y: 2 });
    let mut v = vec![1, 2, 3];
    println!("{} {v:?}", core::mem::take(&mut v));
    println!("{} {:?}", std::mem::take(&mut v).len(), v);
    println!("{} {}", core::hint::black_box(1), std::hint::black_box(1));
    println!("{:?} {:?}", core::iter::once(4), std::iter::once(4).collect::<Vec<_>>());
    println!("{} {}", core::any::type_name::<i32>(), std::any::type_name::<i32>());
    println!("{:?} {:?}", core::char::from_digit(7, 10), std::char::from_digit(7, 10));
    let names: Vec<String> = ["a", "b"].iter().map(|s| s.to_uppercase()).collect();
    println!("{}", names.join("-"));
    let boxed = core::boxed();
    println!("{} {}", core::is_std_option(&*boxed), core::is_std_option(&Some(1)));
}
