use std::io::Write;

use arrayvec::{ArrayString, ArrayVec};

pub fn report() -> String {
    let mut lines = Vec::new();

    let mut v: ArrayVec<i32, 6> = ArrayVec::new();
    for n in [5, 1, 4] {
        v.push(n);
    }
    v.insert(1, 9);
    lines.push(format!("{:?} {} {} {}", v, v.len(), v.capacity(), v.is_full()));
    let removed = v.remove(0);
    let popped = v.pop();
    lines.push(format!("{removed} {popped:?} {v:?}"));
    v.extend([7, 8, 2]);
    v.retain(|n| *n % 2 == 0);
    lines.push(format!("{v:?} {:?}", v.try_push(10)));
    let drained: Vec<i32> = v.drain(..1).collect();
    lines.push(format!("{drained:?} {v:?}"));
    let full: ArrayVec<u8, 2> = [1, 2].into();
    lines.push(format!("{:?} {:?}", full, full.clone().into_inner()));
    let collected: ArrayVec<String, 4> = ["a", "b"].iter().map(|s| s.to_uppercase()).collect();
    lines.push(format!(
        "{collected:?} {:?}",
        collected.iter().rev().collect::<Vec<_>>()
    ));
    let mut words: ArrayVec<String, 3> = ArrayVec::new();
    words.push("x".into());
    words.truncate(0);
    lines.push(format!("{words:?} {}", words.is_empty()));

    let mut s: ArrayString<8> = ArrayString::new();
    s.push_str("hé");
    s.push('!');
    let overflow = s.try_push_str("toolong").is_err();
    lines.push(format!("{s} {} {overflow} {:?}", s.len(), s.pop()));
    s.truncate(1);
    lines.push(format!("{:?} {}", s.as_str(), s == *"h"));

    let mut bytes: ArrayVec<u8, 16> = ArrayVec::new();
    write!(bytes, "n={}", 42).unwrap_or(());
    bytes.write_all(b";ok").unwrap_or(());
    lines.push(String::from_utf8_lossy(&bytes).into_owned());

    lines.join("\n")
}
