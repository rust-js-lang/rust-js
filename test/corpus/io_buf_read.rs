//@ library-differs
// `io::BufRead`'s provided methods (#1156), std's own bodies over a byte
// source's dictionary: `read_line`, `read_until`, `skip_until`, and the
// iterators `lines` and `split`, of a `&[u8]`, a `Cursor`, a generic `R:
// BufRead`, a `&mut` to one and a `dyn BufRead`. A line that isn't UTF-8
// is an error, and nothing of it is added.
use std::io::{BufRead, Cursor};

fn count_lines<R: BufRead>(r: R) -> usize {
    r.lines().count()
}

fn first_line(r: &mut dyn BufRead) -> String {
    let mut s = String::new();
    r.read_line(&mut s).unwrap();
    s
}

fn fields<R: BufRead>(r: &mut R) -> Vec<String> {
    let mut out = Vec::new();
    let mut field = Vec::new();
    while r.read_until(b';', &mut field).unwrap() > 0 {
        out.push(String::from_utf8_lossy(&field).into_owned());
        field.clear();
    }
    out
}

fn main() {
    let mut text = Cursor::new("a\nbb\r\n\nccc");
    let mut line = String::new();
    while text.read_line(&mut line).unwrap() > 0 {
        println!("{line:?}");
        line.clear();
    }
    for line in Cursor::new("one\r\ntwo\n\nthree").lines() {
        println!("{:?}", line.unwrap());
    }
    let parts: Vec<Vec<u8>> = BufRead::split(&b"1,22,,333,"[..], b',')
        .map(|part| part.unwrap())
        .collect();
    println!("{parts:?}");

    let mut c = Cursor::new(String::from("x\ny\n"));
    println!("{} {}", count_lines(&mut c), c.position());
    println!("{}", count_lines(&mut c));
    let bytes: &[u8] = b"first\nsecond";
    println!("{:?}", first_line(&mut Cursor::new(bytes)));
    let mut slice: &[u8] = b"only";
    println!("{:?} {slice:?}", first_line(&mut slice));
    println!("{:?}", fields(&mut &b"k=v;a;;end"[..]));

    let mut skip: &[u8] = b"skip;rest";
    println!("{:?} {skip:?}", skip.skip_until(b';').unwrap());
    println!("{:?} {skip:?}", skip.skip_until(b';').unwrap());

    let mut bad: &[u8] = &[b'o', b'k', 0xff, b'\n', b'z'];
    let mut kept = String::from("kept");
    let e = bad.read_line(&mut kept).unwrap_err();
    println!("{e} {kept:?} {bad:?}");
    let lines: Vec<String> = Cursor::new(vec![b'a', b'\n', 0xc3, b'\n', b'b'])
        .lines()
        .map(|line| line.unwrap_or_else(|e| e.to_string()))
        .collect();
    println!("{lines:?}");
}
