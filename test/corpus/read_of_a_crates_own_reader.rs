// A reader of the crate's own, an `io::Read` impl writing only `read`, has
// std's provided methods over it: `read_exact`, a loop of `read` that tries
// an interrupted read again and fails past the end; `read_to_end` and
// `read_to_string`, alloc's `default_read_to_end`, whose reads the crate's
// `read` sees, a 32-byte probe, then the spare room, capped by a
// `max_read_size` that doubles after a read that fills it; `bytes()`,
// `take` and `chain`. Generic code and a `dyn Read` call them too, and a
// reader that writes its own `read_to_end` keeps it. A `String` it reads
// into is `String::new()` with nothing done to it before, whose capacity
// is 0: another's, which a JS string doesn't carry, is refused (#1297).
use std::io::{self, Read};

/// Gives `left` bytes, at most `most` a read, saying how much it's asked.
struct Counted {
    left: usize,
    most: usize,
    next: u8,
    log: bool,
}

impl Read for Counted {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let n = buf.len().min(self.left).min(self.most);
        if self.log {
            print!(" {}->{}", buf.len(), n);
        }
        for b in &mut buf[..n] {
            *b = self.next;
            self.next = if self.next == b'z' { b'a' } else { self.next + 1 };
        }
        self.left -= n;
        Ok(n)
    }
}

fn counted(left: usize, most: usize) -> Counted {
    Counted {
        left,
        most,
        next: b'a',
        log: true,
    }
}

/// Fails as it's told to, once each, before giving its bytes.
struct Flaky<'a> {
    errors: Vec<io::ErrorKind>,
    bytes: &'a [u8],
}

impl Read for Flaky<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(kind) = self.errors.pop() {
            return Err(io::Error::new(kind, "flaky"));
        }
        let n = buf.len().min(self.bytes.len()).min(2);
        buf[..n].copy_from_slice(&self.bytes[..n]);
        self.bytes = &self.bytes[n..];
        Ok(n)
    }
}

/// Writes its own `read_to_end`, which std's callers use.
struct Whole;

impl Read for Whole {
    fn read(&mut self, _: &mut [u8]) -> io::Result<usize> {
        Ok(0)
    }

    fn read_to_end(&mut self, buf: &mut Vec<u8>) -> io::Result<usize> {
        buf.extend_from_slice(b"whole");
        Ok(5)
    }
}

fn slurp<R: Read>(mut reader: R) -> io::Result<String> {
    let mut s = String::new();
    reader.read_to_string(&mut s)?;
    Ok(s)
}

fn first_three(reader: &mut dyn Read) -> io::Result<[u8; 3]> {
    let mut buf = [0; 3];
    reader.read_exact(&mut buf)?;
    Ok(buf)
}

fn main() {
    for (left, most) in [
        (0, 99),
        (5, 99),
        (32, 99),
        (33, 99),
        (40, 99),
        (100, 7),
        (300, 99),
        (9000, 99),
        (20000, 5000),
        (30000, 99999),
    ] {
        print!("read_to_end {left} by {most}:");
        let mut v = Vec::new();
        let read = counted(left, most).read_to_end(&mut v);
        println!(
            " = {:?} len {} cap {}",
            read.map_err(|e| e.kind()),
            v.len(),
            v.capacity()
        );
    }
    for (room, left) in [(10, 4), (10, 10), (10, 40), (64, 100)] {
        print!("into {room} spare, {left}:");
        let mut v = Vec::with_capacity(room);
        let read = counted(left, 99).read_to_end(&mut v);
        println!(
            " = {:?} len {} cap {}",
            read.map_err(|e| e.kind()),
            v.len(),
            v.capacity()
        );
    }
    print!("read_to_string:");
    let mut s = String::new();
    let read = counted(30, 4).read_to_string(&mut s);
    println!(" = {:?} {s}", read.map_err(|e| e.kind()));
    let mut s = String::default();
    let bad = Flaky {
        errors: vec![],
        bytes: b"ok\xff!",
    }
    .read_to_string(&mut s);
    println!("invalid: {:?} {s:?}", bad.map_err(|e| e.kind()));
    print!("borrowed unused:");
    let mut unused = String::new();
    {
        let _nothing = &mut unused;
    }
    let read = counted(6, 99).read_to_string(&mut unused);
    println!(" = {:?} {unused}", read.map_err(|e| e.kind()));
    print!("boxed:");
    let mut boxed = Box::new(String::new());
    let read = counted(4, 99).read_to_string(&mut *boxed);
    println!(" = {:?} {boxed}", read.map_err(|e| e.kind()));
    print!("held:");
    let mut held = (String::new(), 7);
    let read = counted(5, 99).read_to_string(&mut held.0);
    println!(" = {:?} {} {}", read.map_err(|e| e.kind()), held.0, held.1);
    print!("take's:");
    let mut s = String::new();
    let read = counted(50, 99).take(40).read_to_string(&mut s);
    println!(" = {:?} {}", read.map_err(|e| e.kind()), s.len());

    let mut buf = [0; 5];
    let read = Flaky {
        errors: vec![io::ErrorKind::Interrupted],
        bytes: b"hello!",
    }
    .read_exact(&mut buf);
    println!("read_exact: {read:?} {:?}", std::str::from_utf8(&buf).unwrap());
    let mut buf = [0; 5];
    let read = Flaky {
        errors: vec![],
        bytes: b"hi",
    }
    .read_exact(&mut buf);
    println!(
        "past the end: {:?} {buf:?}",
        read.map_err(|e| (e.kind(), e.to_string()))
    );
    let mut buf = [0; 5];
    let read = Flaky {
        errors: vec![io::ErrorKind::Other],
        bytes: b"hello",
    }
    .read_exact(&mut buf);
    println!("failing: {:?}", read.map_err(|e| (e.kind(), e.to_string())));

    let mut v = Vec::new();
    let read = Flaky {
        errors: vec![io::ErrorKind::Other, io::ErrorKind::Interrupted],
        bytes: b"abcde",
    }
    .read_to_end(&mut v);
    println!("read_to_end failing: {:?} {v:?}", read.map_err(|e| e.kind()));

    let bytes: Vec<u8> = Counted {
        left: 4,
        most: 99,
        next: b'w',
        log: false,
    }
    .bytes()
    .map(Result::unwrap)
    .collect();
    println!("bytes: {bytes:?}");
    print!("take:");
    let mut v = Vec::new();
    let read = counted(100, 99).take(40).read_to_end(&mut v);
    println!(
        " = {:?} len {} cap {}",
        read.map_err(|e| e.kind()),
        v.len(),
        v.capacity()
    );
    print!("chain:");
    let mut v = Vec::new();
    let read = counted(3, 99).chain(counted(40, 99)).read_to_end(&mut v);
    println!(
        " = {:?} len {} cap {}",
        read.map_err(|e| e.kind()),
        v.len(),
        v.capacity()
    );

    print!("generic:");
    println!(" {:?}", slurp(counted(6, 99)).map_err(|e| e.kind()));
    let mut reader = counted(10, 99);
    print!("dyn:");
    println!(" {:?}", first_three(&mut reader).map_err(|e| e.kind()));
    let mut v = Vec::new();
    println!(
        "its own: {:?} {:?}",
        Whole.read_to_end(&mut v).map_err(|e| e.kind()),
        String::from_utf8(v)
    );
    let mut s = String::new();
    println!(
        "its own string: {:?} {s:?}",
        Whole.read_to_string(&mut s).map_err(|e| e.kind())
    );
    println!("{}", slurp(Whole).unwrap());
}
