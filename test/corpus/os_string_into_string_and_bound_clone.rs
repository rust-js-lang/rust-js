// quickcheck 0.9's `OsString` and `Bound<T>` shrinkers (#1420): an
// `OsString`'s `into_string()`, `Ok` of its text, which every `OsString`
// rust-js makes is; and a clone of a `Bound`, std's derived `Clone`, each
// variant with its value's clone.
use std::ffi::OsString;
use std::ops::Bound;

fn shrunk(b: &Bound<String>) -> Bound<String> {
    match b.clone() {
        Bound::Included(mut s) | Bound::Excluded(mut s) => {
            s.pop();
            Bound::Included(s)
        }
        Bound::Unbounded => Bound::Unbounded,
    }
}

fn main() {
    let s = OsString::from("héllo");
    let text: String = s.clone().into_string().unwrap();
    println!("{text} {:?}", s.into_string());
    let owned = OsString::from(String::from("ab"));
    println!("{:?}", owned.into_string().map(|t| t.len()));

    let bounds = vec![
        Bound::Included(String::from("ab")),
        Bound::Excluded(String::from("cd")),
        Bound::Unbounded,
    ];
    let mut copies = bounds.clone();
    if let Bound::Included(s) = &mut copies[0] {
        s.push('!');
    }
    println!("{bounds:?} {copies:?}");
    println!("{:?}", bounds.iter().map(shrunk).collect::<Vec<_>>());
    let n: Bound<u32> = Bound::Excluded(3);
    let m = n;
    println!("{n:?} {m:?} {}", n == m);
}
