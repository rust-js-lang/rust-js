//@ library-differs
// std's conversions where the Crater's crates meet them (#1284): an
// `Into<Vec<u8>>` bound given a slice, text or a vector (regex-syntax's
// `Literal::exact`), a `TryFrom<&[u8]>` bound given an array or the
// crate's type (httparse's `peek_n`), `T::try_from` of a bound, a
// `Box<[T]>` into a `Box<[T; N]>` (miniz_oxide's buffers), `char::try_from`
// of a byte, and an `Into<Box<dyn Error + Send + Sync>>` bound given a
// message (reqwest's); an `OsString`'s `PathBuf`, `.map(PathBuf::from)`
// (autocfg's).
use std::error::Error;
use std::ffi::OsString;
use std::path::PathBuf;

#[derive(Debug)]
struct Two(u8, u8);

impl TryFrom<&[u8]> for Two {
    type Error = ();

    fn try_from(b: &[u8]) -> Result<Self, ()> {
        match b {
            [x, y] => Ok(Two(*x, *y)),
            _ => Err(()),
        }
    }
}

fn exact<B: Into<Vec<u8>>>(bytes: B) -> Vec<u8> {
    bytes.into()
}

fn peek_n<'a, U: TryFrom<&'a [u8]>>(bytes: &'a [u8], n: usize) -> Option<U> {
    bytes.get(..n)?.try_into().ok()
}

fn narrow<T: TryFrom<i64>>(x: i64) -> Option<T> {
    T::try_from(x).ok()
}

fn shrink<T: TryInto<u8>>(t: T) -> Option<u8> {
    t.try_into().ok()
}

fn boxed<E: Into<Box<dyn Error + Send + Sync>>>(e: E) -> String {
    let b: Box<dyn Error + Send + Sync> = e.into();
    format!("{b}")
}

fn main() {
    let mut v = exact(&b"ab"[..]);
    v.push(0);
    println!("{v:?} {:?} {:?} {:?} {:?}", exact(String::from("cd")), exact("é"), exact(vec![1u8]), exact([7u8, 8]));

    let two: Option<[u8; 2]> = peek_n(b"xyz", 2);
    let none: Option<[u8; 4]> = peek_n(b"xyz", 3);
    let own: Option<Two> = peek_n(b"xyz", 2);
    let short: Option<Two> = peek_n(b"xyz", 1);
    let own = own.map(|Two(x, y)| u16::from(x) + u16::from(y));
    println!("{two:?} {none:?} {own:?} {short:?}");
    println!("{:?} {:?} {:?}", narrow::<u8>(300), narrow::<u8>(7), narrow::<i16>(-5));
    println!("{:?} {:?} {:?}", shrink(300i64), shrink(9u16), shrink('a'));

    let b: Box<[u16; 4]> = vec![1u16, 2, 3, 4].into_boxed_slice().try_into().unwrap();
    let bad: Result<Box<[u16; 3]>, Box<[u16]>> = vec![0u16; 4].into_boxed_slice().try_into();
    println!("{b:?} {:?}", bad.map_err(|back| back.len()));

    println!("{:?} {:?} {:?}", char::try_from(65u8), char::try_from(0xE9u8), char::try_from(0x1F600u32).map(char::len_utf8));
    println!("{}", boxed("bad scheme"));
    let dir: Option<OsString> = Some(OsString::from("out"));
    let path = dir.map(PathBuf::from).unwrap_or_else(|| PathBuf::from("target"));
    let back: OsString = path.clone().into();
    println!("{} {:?}", path.display(), back);
}
