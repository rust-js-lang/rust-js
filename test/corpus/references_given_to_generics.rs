//@ library-differs
// A reference given where a generic parameter is: std's impls for `&T` pass
// each trait on to what it refers to, which is the value itself in JS, so
// its dictionary is that one's; std's `AsRef` of a path or text as a
// `Path` is the value as at a call; and a `&mut` to a range is the range,
// stepped where it is.

use std::collections::hash_map::DefaultHasher;
use std::fmt::{Debug, Display, Write};
use std::hash::{Hash, Hasher};
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone)]
struct K(u8, &'static str);
impl Display for K {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "<{}:{}>", self.0, self.1)
    }
}
fn show<T: Display>(t: T) -> String {
    format!("[{t}]")
}
fn dbg<T: Debug>(t: T) -> String {
    format!("{t:?}")
}
fn same<T: PartialEq>(a: T, b: T) -> bool {
    a == b
}
fn biggest<T: Ord>(v: Vec<T>) -> Option<T> {
    v.into_iter().max()
}
fn hashed<T: Hash>(t: T) -> u64 {
    let mut h = DefaultHasher::new();
    t.hash(&mut h);
    h.finish()
}
fn take2<I: Iterator<Item = u32>>(it: I) -> Vec<u32> {
    it.take(2).collect()
}
fn twice<F: Fn(u32) -> u32>(f: F) -> u32 {
    f(f(1))
}
fn fill<W: Write>(mut w: W) -> W {
    write!(w, "x={}", 1).unwrap();
    w
}
fn path_len<P: AsRef<std::path::Path>>(p: P) -> usize {
    p.as_ref().to_str().unwrap().len()
}
fn main() {
    let a = K(1, "a");
    let b = K(2, "b");
    println!("{} {} {}", show(&a), show(&&b), dbg(&a));
    println!("{} {}", same(&a, &a.clone()), same(&&a, &&b));
    println!("{:?}", biggest(vec![&a, &b, &a]));
    println!("{}", hashed(&a) == hashed(a.clone()));
    let mut it = 1..10u32;
    println!("{:?} {:?}", take2(&mut it), it.next());
    let add = |x: u32| x + 5;
    println!("{} {}", twice(&add), twice(add));
    let mut s = String::new();
    fill(&mut s);
    println!("{s}");
    let p = std::path::PathBuf::from("a/b/c");
    println!("{} {}", path_len(&p), path_len("x/y"));
    let names = vec![String::from("bob"), String::from("al")];
    let refs: Vec<&String> = names.iter().collect();
    println!("{:?}", biggest(refs));
    println!("{}", show(&&&5));
    let mut r = 5..=9u32;
    println!("{:?} {:?}", take2(r.by_ref()), r.next());
    println!("{} {}", shout(&String::from("ref")), shout(&&"twice"));
    let t = Text("own".into());
    println!("{} {}", shout(&t), shout(&&t));
}

struct Text(String);

impl AsRef<str> for Text {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

fn shout<S: AsRef<str>>(s: S) -> String {
    s.as_ref().to_uppercase()
}
