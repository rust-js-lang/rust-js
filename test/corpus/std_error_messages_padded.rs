// std's errors whose `Display` is `pad` of a message are padded and cut
// by the placeholder's width and precision, as `pad` does; those that
// `write!` theirs aren't (#1689). Shown directly, through a `dyn Error`,
// and through a writer of the crate's own given its `Formatter`'s options.
use std::convert::TryInto;
use std::error::Error;
use std::ffi::CString;
use std::fmt;
use std::io;

struct Framed<'a>(&'a dyn Error);

impl fmt::Display for Framed<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<")?;
        fmt::Display::fmt(self.0, f)?;
        write!(f, ">")
    }
}

fn main() {
    let boom: Box<dyn Error> = "boom".into();
    println!("[{:>8}] [{:.2}] [{:-^9}] [{}]", boom, boom, boom, boom);
    let owned: Box<dyn Error + Send + Sync> = String::from("bang").into();
    println!("[{:<7}]", owned);

    let other = io::Error::other("oops");
    let made = io::Error::new(io::ErrorKind::NotFound, "gone");
    let kind = io::Error::from(io::ErrorKind::InvalidData);
    println!("[{:>8}] [{:.2}] [{:>8}] [{:>16}]", other, other, made, kind);
    let mut buffer = [0u8; 4];
    let short = io::Read::read_exact(&mut &b"ab"[..], &mut buffer).unwrap_err();
    println!("[{:>30}] [{:.6}]", short, short);
    let held: Box<dyn Error> = Box::new(io::Error::other("in"));
    println!("[{:>6}] {:?}", held, held);

    let slice: Result<[u8; 2], _> = [1u8, 2, 3][..].try_into();
    println!("[{:>35}]", slice.unwrap_err());
    println!("[{:>50}] [{:.5}]", fmt::Error, fmt::Error);
    let utf16 = String::from_utf16(&[0xd800]).unwrap_err();
    println!("[{:^40}]", utf16);
    let prefix = std::path::Path::new("/a").strip_prefix("/b").unwrap_err();
    println!("[{:>20}]", prefix);
    let not_utf8 = CString::new(vec![0xffu8]).unwrap().into_string().unwrap_err();
    println!("[{:>40}]", not_utf8);
    let nul = CString::new(vec![b'a', 0]).unwrap_err();
    println!("[{:>50}]", nul);
    let lock = std::sync::Mutex::new(1);
    let _held = lock.lock().unwrap();
    let blocked = lock.try_lock().unwrap_err();
    println!("[{:>55}]", blocked);

    println!("[{:>8}] [{:>8}]", Framed(&*held), Framed(&other));
    println!("[{:>10}]", Framed(&*boom));
}
