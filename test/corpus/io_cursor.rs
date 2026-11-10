//@ library-differs
// `io::Cursor<T>` (#1156): std's struct, `{ inner, pos }`, read at its
// position of its inner value's bytes, of each `T` that's `AsRef<[u8]>`;
// past its end there's nothing. Its `Seek` moves it, wrapping as std's
// build does, and refuses a position below 0; `rewind`, `seek_relative`,
// `Clone`, `Debug`, `PartialEq` and `Default` are std's own bodies.
use std::io::{BufRead, Cursor, Read, Seek, SeekFrom};

fn main() {
    let mut c = Cursor::new(vec![10u8, 20, 30, 40]);
    let mut three = [0u8; 3];
    c.read_exact(&mut three).unwrap();
    println!("{three:?} {}", c.position());
    let e = c.read_exact(&mut three).unwrap_err();
    println!("{e} {} {three:?}", c.position());
    c.set_position(1);
    let mut rest = Vec::new();
    println!("{:?} {rest:?} {}", c.read_to_end(&mut rest).unwrap(), c.position());

    println!("{:?}", c.seek(SeekFrom::Start(2)).unwrap());
    println!("{:?}", c.seek(SeekFrom::Current(-1)).unwrap());
    println!("{:?}", c.seek(SeekFrom::End(-3)).unwrap());
    let e = c.seek(SeekFrom::End(-5)).unwrap_err();
    println!("{:?} {e:?} {}", e.kind(), c.position());
    println!("{:?}", c.seek(SeekFrom::End(6)).unwrap());
    let mut b = [9u8; 2];
    println!("{:?} {b:?} {}", c.read(&mut b).unwrap(), c.position());
    let mut s = String::new();
    println!("{:?} {s:?}", c.read_to_string(&mut s).unwrap());
    c.rewind().unwrap();
    c.seek_relative(3).unwrap();
    println!("{:?} {:?}", c.stream_position().unwrap(), c.fill_buf().unwrap());
    c.seek(SeekFrom::Start(u64::MAX)).unwrap();
    c.consume(2);
    println!("{} {:?}", c.position(), c.seek(SeekFrom::Current(i64::MAX)).is_err());

    let mut text = Cursor::new("héllo\n");
    let mut word = String::new();
    text.read_to_string(&mut word).unwrap();
    println!("{word:?} {}", text.into_inner());
    let owned = String::from("xy");
    let mut by_ref = Cursor::new(&owned);
    let mut two = [0u8; 2];
    by_ref.read_exact(&mut two).unwrap();
    println!("{two:?} {}", by_ref.get_ref());
    let mut array = Cursor::new([5u8; 3]);
    println!("{:?}", array.read(&mut [0u8; 8]).unwrap());
    let mut boxed = Cursor::new(vec![1u8].into_boxed_slice());
    println!("{:?}", boxed.read_exact(&mut two).is_err());
    let mut v = vec![7u8, 8, 9];
    let mut through = Cursor::new(&mut v);
    through.read_exact(&mut two).unwrap();
    println!("{two:?} {:?}", through.get_ref());
    let bytes: &[u8] = b"ab";
    let mut of_slice = Cursor::new(bytes);
    println!("{:?}", of_slice.read(&mut [0u8; 1]).unwrap());

    let d = of_slice.clone();
    println!("{of_slice:?} {d:#?} {}", of_slice == d);
    let mut e = Cursor::new(String::from("x"));
    e.clone_from(&Cursor::new(String::from("yz")));
    let empty: Cursor<Vec<u8>> = Cursor::default();
    println!("{e:?} {empty:?} {}", empty == Cursor::new(Vec::new()));
}
