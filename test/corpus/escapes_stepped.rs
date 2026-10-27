// An escape is the array of what it gives, as a case mapping is: a text
// escape's `char`s, a byte slice's `escape_ascii()`'s bytes. Held, it's
// std's iterator of them, so `next()` steps it and `{}` shows what's left
// (ADR 0971, #379).
fn main() {
    let mut e = 'é'.escape_default();
    let first = e.next();
    let rest: String = e.collect();
    println!("{first:?} {rest}");
    let mut u = "ab".escape_debug();
    println!("{:?} {u}", u.next());
    let mut w = '\t'.escape_unicode();
    w.next();
    w.next();
    println!("{} {w}", w.len());

    let bytes = b"a\n\xff";
    let v: Vec<u8> = bytes.escape_ascii().collect();
    println!("{v:?}");
    let n = bytes.escape_ascii().count();
    let s: u32 = bytes.escape_ascii().map(u32::from).sum();
    println!("{n} {s} {}", bytes.escape_ascii());
    let mut held = bytes.escape_ascii();
    println!("{:?} {:?} {held}", held.next(), held.next_back());

    // A case mapping, the same array of its `char`s, knows its length.
    let mut upper = 'ß'.to_uppercase();
    upper.next();
    println!("{} {upper}", upper.len());
    let mut lower = 'İ'.to_lowercase();
    println!("{:?} {lower}", lower.next_back());
}
