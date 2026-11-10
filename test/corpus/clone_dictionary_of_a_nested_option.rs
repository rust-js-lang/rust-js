// Generic code's `Clone` of an `Option<Option<T>>` whose `Some` is boxed,
// each given its own dictionary: its `clone(value)`'s binding of what the
// outer `Some` holds is a name of its own, never its parameter's, which a
// second `const value` in its body would declare again (#1246).
fn cloned<T: Clone>(x: &T) -> T {
    x.clone()
}

fn copied_out<T: Clone>(x: &T) -> T {
    x.clone()
}

fn defaulted<T: Clone>(x: &T) -> T {
    x.clone()
}

fn fallen_back<T: Clone>(x: &T) -> T {
    x.clone()
}

fn unwraps<T: Clone>(a: Option<Option<T>>, r: Result<Option<T>, u8>) -> [Option<T>; 5] {
    [
        cloned(&a).unwrap(),
        copied_out(&a).unwrap_or(None),
        defaulted(&a).unwrap_or_default(),
        fallen_back(&r).unwrap_or_else(|_| None),
        r.unwrap_or_default(),
    ]
}

fn main() {
    println!("{:?}", unwraps(Some(Some(Some(5u8))), Err(1)));
    println!("{:?}", unwraps(Some(None::<Option<u8>>), Ok(Some(None))));
    println!("{:?}", unwraps::<String>(Some(Some(String::from("s"))), Ok(None)));
}
