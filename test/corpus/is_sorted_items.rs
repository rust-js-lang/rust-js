// is_sorted() of a slice's or an iterator's items, by their PartialOrd: a
// NaN isn't at most anything, a char is by code point, and an iterator
// stops at the first pair out of order.
#[derive(Debug, PartialEq, PartialOrd)]
enum L {
    A,
    B(f64),
}
fn main() {
    let t = |x: i32| {
        println!("t{x}");
        x
    };
    println!("{} {}", [1, 2, 2].is_sorted(), [3, 1].iter().is_sorted());
    println!("{} {}", [1.0, f64::NAN].is_sorted(), [0.5, 1.5].is_sorted());
    println!("{} {}", ["a", "b", "B"].is_sorted(), ['a', '😀'].is_sorted());
    println!("{} {}", [L::A, L::B(1.0), L::B(2.0)].is_sorted(), [L::B(f64::NAN), L::B(1.0)].is_sorted());
    println!("{}", (1..10).map(t).map(|x| if x == 3 { 0 } else { x }).is_sorted());
    println!("{} {}", Vec::<u8>::new().is_sorted(), [true, false].is_sorted());
    println!("{}", vec![String::from("x"), String::from("y")].iter().is_sorted());
}
