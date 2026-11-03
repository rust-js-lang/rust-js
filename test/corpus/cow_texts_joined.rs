// A `String` made of `Cow<str>`s, by `collect`, `from_iter`, `extend`, a
// slice's `concat` and `join`, and in generic code: each one's `str`, its
// `_0`, as std pushes what it borrows or owns (#495).
use std::borrow::Cow;

fn cows() -> Vec<Cow<'static, str>> {
    vec![Cow::Borrowed("a"), Cow::Owned("b".to_string())]
}

fn generic<I: Iterator<Item = Cow<'static, str>>>(it: I) -> String {
    it.collect()
}

fn main() {
    let s: String = cows().into_iter().collect();
    println!("{s}");
    let s = String::from_iter(cows());
    println!("{s}");
    let mut t = String::from("x");
    t.extend(cows());
    println!("{t}");
    println!("{}", cows().concat());
    println!("{}", cows().join(","));
    println!("{}", generic(cows().into_iter()));
    let s: String = cows().iter().map(|c| c.clone()).collect();
    println!("{s}");
    let s: String = cows().into_iter().rev().collect();
    println!("{s}");
}
