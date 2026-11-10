//@ library-differs
// A generic `R: io::Read` (#1156), given its source's dictionary; a `dyn
// Read`, the pair; a `Box<dyn Read>`, and a `&mut` to a reader, through
// std's forwarding impls. A `read_exact` past the end is an error `?`
// gives on, a `Box<dyn Error>` of it too.
use std::error::Error;
use std::io::{self, Cursor, Read};

fn first<R: Read>(r: &mut R) -> io::Result<u8> {
    let mut b = [0u8; 1];
    r.read_exact(&mut b)?;
    Ok(b[0])
}

fn pair(r: &mut impl Read) -> Result<[u8; 2], Box<dyn Error>> {
    let mut b = [0u8; 2];
    r.read_exact(&mut b)?;
    Ok(b)
}

fn total(r: &mut dyn Read) -> io::Result<u32> {
    let mut all = Vec::new();
    r.read_to_end(&mut all)?;
    Ok(all.iter().map(|&b| u32::from(b)).sum())
}

fn skip<R: Read>(r: &mut R, n: usize) -> io::Result<()> {
    let mut b = vec![0u8; n];
    r.read_exact(&mut b)
}

fn skip_four(r: &mut &[u8]) -> io::Result<()> {
    r.read_exact(&mut [0u8; 4])
}

fn text_of<R: Read>(mut r: R) -> String {
    let mut s = String::new();
    r.read_to_string(&mut s).unwrap();
    s
}

fn main() {
    let data = [7u8, 8];
    let mut s: &[u8] = &data;
    println!("{:?} {:?} {:?} {s:?}", first(&mut s), first(&mut s), first(&mut s).map_err(|e| e.kind()));
    let mut c = Cursor::new(vec![1u8, 2]);
    println!("{:?} {:?} {:?} {}", first(&mut c), first(&mut c), first(&mut c).is_err(), c.position());
    c.set_position(0);
    println!("{:?}", text_of(&mut c));
    println!("{:?}", pair(&mut Cursor::new("hi")).map_err(|e| e.to_string()));
    println!("{:?}", pair(&mut &b"x"[..]).map_err(|e| e.to_string()));
    let mut d = Cursor::new(vec![5u8; 7]);
    d.set_position(3);
    println!("{:?}", total(&mut d));
    let mut boxed: Box<dyn Read> = Box::new(Cursor::new(vec![1u8, 1, 1]));
    println!("{:?} {:?}", total(&mut boxed), first(&mut boxed).is_err());
    println!("{:?}", text_of(&b"by value"[..]));
    let mut few: &[u8] = b"ab";
    println!("{:?} {few:?}", skip(&mut few, 3).map_err(|e| e.kind()));
    let mut three: &[u8] = b"abc";
    println!("{:?} {three:?}", skip_four(&mut three).is_err());
    println!("{:?}", text_of(Cursor::new(String::from("owned"))));
}
