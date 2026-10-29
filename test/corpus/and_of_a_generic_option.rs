// `and` of a type parameter's `Option`, which could be an `Option` itself:
// the `Option` it's given, boxed as it was given, never refused (ADR 0051).
fn pick<T: std::fmt::Debug>(a: Option<u8>, b: Option<T>) -> Option<T> {
    a.and(b)
}

fn main() {
    println!("{:?}", pick(Some(1), Some(Some(2))));
    println!("{:?}", pick(Some(1), Some(None::<u8>)));
    println!("{:?}", pick(None, Some(Some(3))));
    println!("{:?}", pick(Some(1), None::<Option<u8>>));
    println!("{:?}", pick(Some(4), Some("text")));
}
