// #118's odds and ends of strings: `fmt::Error`'s display, a
// `ParseIntError`'s clone, `Chars`' `{:?}`, `CharIndices`' `offset` and `as_str`,
// `char::encode_utf16` and a string `Drain`'s `as_str`, `next_back` and
// `{:?}`.

use std::fmt;

fn main() {
    println!("{}", fmt::Error);
    println!("{:?}", fmt::Error);

    let e = "x".parse::<i32>().unwrap_err();
    let c = e.clone();
    println!("{c} {:?} {}", c.kind(), e == c);

    let mut chars = "héllo".chars();
    chars.next();
    println!("{chars:?}");
    println!("{:?}", "".chars());

    let mut it = "aé😀b".char_indices();
    print!("{}", it.offset());
    while it.next().is_some() {
        print!(" {}", it.offset());
    }
    println!();
    let mut it = "aé😀b".char_indices();
    it.next_back();
    it.next();
    println!("{} {:?}", it.offset(), it.as_str());

    let mut buf = [0u16; 2];
    for ch in ['a', 'é', '😀'] {
        let units = ch.encode_utf16(&mut buf);
        println!("{ch} {} {units:?}", units.len());
    }

    let mut s = String::from("héllo wörld");
    let mut d = s.drain(1..7);
    println!("{:?}", d.as_str());
    d.next();
    println!("{:?}", d.as_str());
    let rest: String = d.collect();
    println!("{rest} {s}");

    let mut s = String::from("héllo wörld");
    let mut d = s.drain(1..10);
    println!("{:?} {d:?}", d.next_back());
    let back: String = d.rev().collect();
    println!("{back} {s}");
}
