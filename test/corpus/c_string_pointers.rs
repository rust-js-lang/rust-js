//@ native: wasm32
// A C string's pointer: `as_ptr` read byte by byte as `c_char`s, its sign
// a byte above 127's, `CStr::from_ptr` of it and of a pointer past its
// start, and a `CString` handed out by `into_raw` and taken back by
// `from_raw` (#1291).
use std::ffi::{CStr, CString, c_char};

fn length(mut p: *const c_char) -> usize {
    let mut n = 0;
    unsafe {
        while *p != 0 {
            n += 1;
            p = p.add(1);
        }
    }
    n
}

fn main() {
    let c = c"hi\xc3\xa9!";
    let p = c.as_ptr();
    println!("{}", length(p));
    unsafe {
        println!("{} {} {}", *p, *p.add(2), *p.add(5));
        let again = CStr::from_ptr(p);
        println!("{:?} {}", again, again == c);
        println!("{:?}", CStr::from_ptr(p.add(1)).to_str());
    }

    let owned = CString::new("raw bytes").unwrap();
    let raw = owned.into_raw();
    unsafe {
        *raw = b'R' as c_char;
        println!("{}", length(raw));
        let back = CString::from_raw(raw);
        println!("{back:?}");
        println!("{:?}", back.into_string());
    }
}
