// A `BufReader` of a `&mut` to a reader, a `Cursor`'s and another
// `BufReader`'s: what it reads moves the reader it borrows, which is read on
// after it's gone, and what `get_ref` and `into_inner` give is that reader
// (#1374).
use std::io::{BufRead, BufReader, Cursor, Read};

fn main() {
    let mut inner = Cursor::new(vec![1u8, 2, 3, 4, 5]);
    {
        let mut by_ref = BufReader::with_capacity(2, &mut inner);
        let mut b = [0u8; 1];
        by_ref.read_exact(&mut b).unwrap();
        println!("{:?} {:?} {}", b, by_ref.buffer(), by_ref.get_ref().position());
    }
    println!("{}", inner.position());

    let mut rest = Vec::new();
    inner.read_to_end(&mut rest).unwrap();
    println!("{:?}", rest);

    let mut text = Cursor::new("one\ntwo\nthree\n");
    let mut lines = BufReader::new(&mut text);
    let mut line = String::new();
    lines.read_line(&mut line).unwrap();
    let back = lines.into_inner();
    println!("{:?} {}", line, back.position());
    back.set_position(4);
    let mut tail = String::new();
    text.read_to_string(&mut tail).unwrap();
    println!("{:?}", tail);

    let mut outer = BufReader::with_capacity(3, Cursor::new(b"abcdefg".to_vec()));
    let mut first = [0u8; 2];
    {
        let mut nested = BufReader::with_capacity(1, &mut outer);
        nested.read_exact(&mut first).unwrap();
    }
    let mut after = String::new();
    outer.read_to_string(&mut after).unwrap();
    println!("{:?} {:?}", first, after);

    // A `Cursor` of a `&mut` to a `Vec` holds it as it is.
    let mut bytes = vec![7u8, 8, 9];
    let mut over = Cursor::new(&mut bytes);
    let mut one = [0u8; 1];
    over.read_exact(&mut one).unwrap();
    println!("{:?} {:?} {}", one, over.get_ref(), over.position());
}
