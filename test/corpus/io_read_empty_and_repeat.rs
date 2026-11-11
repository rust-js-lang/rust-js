//@ library-differs
// `io::empty()` and `io::repeat(byte)` read (#1156), as std's impls do:
// an empty reader reads nothing, its `read_exact` of a buffer that isn't
// empty is `UnexpectedEof` and of one that is, `Ok`; every line it reads
// is none and every position it seeks is 0. A repeating one fills each
// buffer with its byte, and reading it to its end is `OutOfMemory`. Each
// known, generic, a `&mut dyn Read` and a `Box<dyn BufRead>`.
use std::io::{self, BufRead, Read, Seek, SeekFrom};

fn first<R: Read>(mut r: R) -> io::Result<u8> {
    let mut b = [0u8];
    r.read_exact(&mut b)?;
    Ok(b[0])
}

fn count_lines<B: BufRead>(b: B) -> usize {
    b.lines().count()
}

fn main() {
    let mut e = io::empty();
    let mut buf = [7u8; 3];
    println!("{:?} {buf:?}", e.read(&mut buf).unwrap());
    let failed = e.read_exact(&mut buf).unwrap_err();
    println!("{:?} {failed}", failed.kind());
    println!("{:?}", e.read_exact(&mut []));
    let mut s = String::from("<");
    println!("{:?} {s:?}", e.read_to_string(&mut s).unwrap());
    let mut v = vec![1u8];
    println!("{:?} {v:?}", e.read_to_end(&mut v).unwrap());
    println!("{:?}", e.fill_buf().unwrap().len());
    e.consume(3);
    println!("{:?} {s:?}", e.read_line(&mut s).unwrap());
    println!("{:?} {v:?}", e.read_until(b'\n', &mut v).unwrap());
    let end = e.seek(SeekFrom::End(-4)).unwrap();
    println!("{end} {}", e.stream_position().unwrap());
    println!("{}", count_lines(io::empty()));
    println!("{:?}", first(io::empty()).map_err(|e| e.kind()));

    let mut r = io::repeat(5);
    println!("{:?} {buf:?}", r.read(&mut buf).unwrap());
    let mut four = [0u8; 4];
    r.read_exact(&mut four).unwrap();
    println!("{four:?}");
    let mut out = Vec::new();
    let failed = r.read_to_end(&mut out).unwrap_err();
    println!("{:?} {failed} {failed:?} {out:?}", failed.kind());
    println!("{:?} {s:?}", r.read_to_string(&mut s).map_err(|e| e.kind()));
    println!("{:?} {:?}", first(io::repeat(9)), first(&mut r));
    let dynamic: &mut dyn Read = &mut r;
    let mut two = [0u8; 2];
    dynamic.read_exact(&mut two).unwrap();
    println!("{two:?}");
    let mut boxed: Box<dyn BufRead> = Box::new(io::empty());
    println!("{}", count_lines(&mut boxed));
}
