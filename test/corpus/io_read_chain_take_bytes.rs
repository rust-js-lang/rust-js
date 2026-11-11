//@ library-differs
// `chain`, `take` and `bytes` of std's byte sources read (#1156), as std's
// impls do: a chain reads its first until it's done, then its second, and
// its `read_to_string` and `read_line` keep a character split between
// them; a take reads at most its limit, its `limit()` what's left; a
// reader's bytes are each byte read, one at a time. Each known, generic, a
// `&mut dyn Read` and a `Box<dyn BufRead>`.
use std::io::{self, BufRead, Read};

fn first_two<R: Read>(mut r: R) -> io::Result<[u8; 2]> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b)?;
    Ok(b)
}

fn lines_of<B: BufRead>(b: B) -> Vec<String> {
    b.lines().map(|line| line.unwrap()).collect()
}

fn until<B: BufRead>(b: &mut B, byte: u8) -> Vec<u8> {
    let mut v = Vec::new();
    b.read_until(byte, &mut v).unwrap();
    v
}

fn main() {
    // A chain of two slices.
    let a: &[u8] = b"ab";
    let b: &[u8] = b"cd\nef";
    let mut chained = a.chain(b);
    let mut buf = [0u8; 4];
    println!("{} {buf:?}", chained.read(&mut buf).unwrap());
    println!("{} {buf:?}", chained.read(&mut buf).unwrap());
    let mut line = String::new();
    println!("{} {line:?}", chained.read_line(&mut line).unwrap());
    let mut rest = String::new();
    println!("{} {rest:?}", chained.read_to_string(&mut rest).unwrap());
    println!("{}", chained.read(&mut buf).unwrap());

    // A character split between the two.
    let left: &[u8] = &[b'x', 0xC3];
    let right: &[u8] = &[0xA9, b'y', b'\n', b'z'];
    let mut s = String::new();
    left.chain(right).read_to_string(&mut s).unwrap();
    println!("{s:?}");
    let mut s = String::new();
    left.chain(right).read_line(&mut s).unwrap();
    println!("{s:?}");
    let bad: &[u8] = &[0xC3];
    let mut s = String::from("kept");
    let failed = bad.chain(&b"!"[..]).read_to_string(&mut s).unwrap_err();
    println!("{:?} {failed} {s:?}", failed.kind());

    let mut v = vec![9u8];
    println!("{} {v:?}", a.chain(b).read_to_end(&mut v).unwrap());
    let mut v = Vec::new();
    println!("{} {v:?}", a.chain(b).read_until(b'd', &mut v).unwrap());
    println!("{:?}", lines_of(a.chain(b)));
    println!("{:?}", first_two((&b"q"[..]).chain(&b"rs"[..])));
    let mut short = a.chain(&b""[..]);
    let mut three = [0u8; 3];
    let failed = short.read_exact(&mut three).unwrap_err();
    println!("{:?} {failed}", failed.kind());
    let (one, two) = a.chain(b).into_inner();
    println!("{one:?} {two:?}");
    // A chain's own `read_until`, of one it's called on again, and through
    // generic code's `&mut B`.
    let mut c = a.chain(b);
    let mut v = Vec::new();
    println!("{} {v:?}", c.read_until(b'c', &mut v).unwrap());
    println!("{} {v:?}", c.read_until(b'\n', &mut v).unwrap());
    println!("{:?} {:?}", until(&mut c, b'z'), until(&mut c, b'z'));
    let mut c = a.chain(b);
    println!("{:?} {:?}", until(&mut c, b'b'), c.fill_buf().unwrap());
    c.consume(1);
    let rest = until(&mut c, b'e');
    println!("{:?} {rest:?}", c.get_ref());

    // A take of a slice, of a repeat, and of a chain.
    let mut taken = (&b"hello"[..]).take(3);
    let mut t = String::new();
    println!("{} {t:?} {}", taken.read_to_string(&mut t).unwrap(), taken.limit());
    taken.set_limit(5);
    let mut t = String::new();
    println!("{} {t:?} {}", taken.read_to_string(&mut t).unwrap(), taken.limit());
    let mut limited = io::repeat(1).take(4);
    let mut out = Vec::new();
    println!("{:?} {out:?} {}", limited.read_to_end(&mut out).unwrap(), limited.limit());
    let mut part = (&b"abcdef"[..]).take(4);
    let mut two = [0u8; 2];
    println!("{} {two:?} {}", part.read(&mut two).unwrap(), part.limit());
    let mut five = [0u8; 5];
    println!("{} {five:?} {}", part.read(&mut five).unwrap(), part.limit());
    println!("{:?}", part.get_ref());
    let mut lines = (&b"one\ntwo\nthree"[..]).take(9);
    let mut l = String::new();
    lines.read_line(&mut l).unwrap();
    lines.read_line(&mut l).unwrap();
    println!("{l:?} {}", lines.limit());
    println!("{:?}", lines_of((&b"x\ny\nz"[..]).take(4)));
    let mut through = a.chain(b).take(3);
    let mut t = String::new();
    through.read_to_string(&mut t).unwrap();
    println!("{t:?}");
    println!("{:?}", first_two(io::repeat(7).take(1)).map_err(|e| e.kind()));
    println!("{:?}", (&b"abc"[..]).take(2).into_inner());
    let mut t = (&b"xyz"[..]).take(2);
    println!("{:?}", t.fill_buf().unwrap());
    t.consume(1);
    println!("{:?}", t.fill_buf().unwrap());
    println!("{:?} {}", until(&mut t, b'q'), t.limit());

    // Bytes, of a slice, a chain and a take.
    let bytes: Vec<u8> = (&b"xyz"[..]).bytes().map(|b| b.unwrap()).collect();
    println!("{bytes:?}");
    let bytes: io::Result<Vec<u8>> = a.chain(b).bytes().collect();
    println!("{bytes:?}");
    println!("{}", io::repeat(3).take(5).bytes().count());
    let mut it = (&b"pq"[..]).bytes();
    println!("{:?} {:?} {:?}", it.next().map(Result::unwrap), it.next().map(Result::unwrap), it.next().is_none());

    // A `&mut dyn Read` and a `Box<dyn BufRead>` of them.
    let mut c = a.chain(b);
    let dynamic: &mut dyn Read = &mut c;
    let mut two = [0u8; 3];
    dynamic.read_exact(&mut two).unwrap();
    println!("{two:?}");
    let mut boxed: Box<dyn BufRead> = Box::new((&b"m\nn"[..]).take(10));
    println!("{:?}", lines_of(&mut boxed));
}
