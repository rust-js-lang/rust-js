// `{:#?}` of std's `TryFromSliceError`, `FromUtf16Error` and
// `DecodeUtf16Error` is on lines of their own, indented, as their derived
// `Debug` writes it, alone and in a `Result`, a `Vec` and a writer's sink:
// the runtime's writer of std's errors writes them, the
// one owner, where the lowering wrote fixed one-line texts (#2510).
use std::char::DecodeUtf16Error;
use std::fmt::{self, Write};

// A writer of its own, whose sink each piece of a derived `Debug` reaches.
struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

#[derive(Debug)]
struct Held {
    slice: std::array::TryFromSliceError,
    utf16: std::string::FromUtf16Error,
    decode: DecodeUtf16Error,
}

fn main() {
    let slice = <[u8; 2]>::try_from(&[1u8][..]);
    let utf16 = String::from_utf16(&[0xD800]);
    let decode = char::decode_utf16([0xD800u16]).next().unwrap();
    println!("{slice:#?}");
    println!("{utf16:#?}");
    println!("{decode:#?}");
    println!("{slice:?} {utf16:?} {decode:?}");
    let errors = vec![slice.unwrap_err()];
    println!("{errors:#?}");
    let mut s = String::new();
    write!(s, "{decode:#?}|{utf16:?}").unwrap();
    println!("{s}");
    let e = String::from_utf16(&[0xDC00]).unwrap_err();
    let d = char::decode_utf16([0xDC00u16]).next().unwrap().unwrap_err();
    println!("{e:#?} {d:#?} {e} {d}");
    let held = Held {
        slice: <[u8; 2]>::try_from(&[1u8][..]).unwrap_err(),
        utf16: e,
        decode: d,
    };
    let mut log = Log(Vec::new());
    write!(log, "{held:?}|{held:#?}").unwrap();
    println!("{}", log.0.concat());
    println!("{} {} {}", held.slice, held.utf16, held.decode.unpaired_surrogate());
}
