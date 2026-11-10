//@ run-fail: range end index 5 out of range for slice of length 1
// A `VecDeque<u8>` read (#1156): `read_exact`, `read_to_end`,
// `read_to_string` and `consume` take what they read from its front, as
// std's impls in alloc do, an array changed in place. One that can't
// fill its buffer empties the deque and fills nothing; text that isn't
// UTF-8 adds nothing and empties it all the same; `consume` past its end
// panics. Its `read` and `fill_buf`, which read its front slice alone,
// are refused (#1302).
use std::collections::VecDeque;
use std::io::{BufRead, Read};

fn main() {
    let mut d: VecDeque<u8> = VecDeque::from(vec![1u8, 2, 3, 4]);
    d.push_front(0);
    let mut two = [0u8; 2];
    d.read_exact(&mut two).unwrap();
    println!("{two:?} {d:?}");
    let mut rest = vec![9u8];
    println!("{:?} {rest:?} {d:?}", d.read_to_end(&mut rest).unwrap());

    let mut short: VecDeque<u8> = VecDeque::from(vec![5u8]);
    let e = short.read_exact(&mut two).unwrap_err();
    println!("{e} {:?} {two:?} {short:?}", e.kind());

    let mut text: VecDeque<u8> = "héllo".bytes().collect();
    let mut s = String::from("> ");
    println!("{:?} {s} {text:?}", text.read_to_string(&mut s).unwrap());
    let mut bad: VecDeque<u8> = VecDeque::from(vec![b'a', 0xff]);
    let mut kept = String::from("kept");
    let e = bad.read_to_string(&mut kept).unwrap_err();
    println!("{e} {kept} {bad:?}");

    let mut by_ref = VecDeque::from(vec![7u8, 8, 9]);
    let through = &mut by_ref;
    through.read_exact(&mut two[..1]).unwrap();
    through.consume(1);
    println!("{two:?} {by_ref:?}");

    let mut c: VecDeque<u8> = VecDeque::from(vec![1u8, 2, 3]);
    c.consume(2);
    println!("{c:?}");
    c.consume(5);
}
