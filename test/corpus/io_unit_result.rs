// An `io::Result<()>`'s `Err` is thrown, as a `fmt::Result`'s is, and caught
// where it's a value (ADR 0462): `?` of a value's `Result` into one, of one
// into a value's, a `match` of one, a crate's writer whose `write` fails
// through std's `write_all`, and its methods.
use std::io::{self, Write};

fn check(n: u32) -> io::Result<()> {
    if n > 3 {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "too big"));
    }
    Ok(())
}

fn parse(s: &str) -> io::Result<u32> {
    s.parse::<u32>().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

// `?` of a value's result into a unit one, and of a unit one.
fn run(s: &str) -> io::Result<()> {
    let n = parse(s)?;
    check(n)?;
    println!("ran {n}");
    Ok(())
}

// Only `?` of a value's `Result`: it fails where that does.
fn parsed_only(s: &str) -> io::Result<()> {
    let n = parse(s)?;
    println!("parsed {n}");
    Ok(())
}

// `?` of a unit result into a value's.
fn doubled(s: &str) -> io::Result<u32> {
    let n = parse(s)?;
    check(n)?;
    Ok(n * 2)
}

struct Limited {
    room: usize,
    out: Vec<u8>,
}

impl Write for Limited {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if self.room == 0 {
            return Err(io::Error::new(io::ErrorKind::WriteZero, "full"));
        }
        let n = bytes.len().min(self.room);
        self.out.extend_from_slice(&bytes[..n]);
        self.room -= n;
        Ok(n)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// Only std's `write_all` of the crate's writer, whose `write` may fail.
fn put(w: &mut Limited) -> io::Result<()> {
    w.write_all(b"xyz")
}

// `?` of one that gives std's `write_all`'s: its error passed on.
fn put_twice(w: &mut Limited) -> io::Result<()> {
    put(w)?;
    put(w)
}

// A `Result`'s method that makes a write's result of a value's: `map`.
fn written(r: io::Result<usize>) -> io::Result<()> {
    r.map(|n| println!("wrote {n}"))
}

fn written_twice(n: usize) -> io::Result<()> {
    written(Ok(n))?;
    written(Err(io::Error::other("refused")))
}

fn main() {
    for s in ["2", "x", "9"] {
        match run(s) {
            Ok(()) => println!("ok"),
            Err(e) => println!("err {e} {:?}", e.kind()),
        }
        println!("{:?} {}", doubled(s).map_err(|e| e.kind()), run(s).is_ok());
    }
    let mut w = Limited { room: 5, out: Vec::new() };
    let a = w.write_all(b"abc");
    let b = w.write_all(b"defg");
    println!("{a:?} {b:?} {:?}", String::from_utf8_lossy(&w.out));
    println!("{:?} {:?}", parsed_only("x").map_err(|e| e.kind()), parsed_only("4").is_ok());
    let mut full = Limited { room: 0, out: Vec::new() };
    println!("{:?}", put(&mut full));
    let mut room = Limited { room: 4, out: Vec::new() };
    println!("{:?} {:?}", put_twice(&mut room), String::from_utf8_lossy(&room.out));
    println!("{:?} {:?}", written(Ok(3)), written_twice(4).map_err(|e| e.to_string()));
    if let Err(e) = check(7) {
        println!("caught {e}");
    }
    println!("{} {:?}", check(1).is_ok(), check(8).map_err(|e| e.to_string()));
}
