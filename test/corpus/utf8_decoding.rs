// Bytes to text, as Rust validates UTF-8: `str::from_utf8`'s `Utf8Error`,
// where the first bad sequence starts and how long it is, `String::from_utf8`,
// and `from_utf8_lossy`, a U+FFFD for each bad sequence, borrowed where
// there's none.

use std::borrow::Cow;

fn show(bytes: &[u8]) {
    match std::str::from_utf8(bytes) {
        Ok(s) => println!("ok {:?} {}", s, s.len()),
        Err(e) => println!("err {} | {:?} | {} {:?}", e, e, e.valid_up_to(), e.error_len()),
    }
}

fn main() {
    let inputs: [&[u8]; 12] = [
        b"plain",
        "h\u{e9}llo \u{20ac}\u{1f600}".as_bytes(),
        b"ab\xffcd",
        b"\xc3",
        b"\xe2\x82",
        b"\xe2\x28\xa1",
        b"\xf0\x9f\x98",
        b"\xed\xa0\x80",
        b"\xc0\x80",
        b"\xf5\x80\x80\x80",
        b"\xf4\x90\x80\x80",
        b"a\xf0\x90\x80\x41",
    ];
    for bytes in inputs {
        show(bytes);
    }

    match String::from_utf8(vec![104, 105, 0xff, 33]) {
        Ok(s) => println!("{}", s),
        Err(e) => {
            println!("{} | {:?} | {:?}", e, e, e.utf8_error());
            println!("{:?}", e.into_bytes());
        }
    }
    println!("{:?}", String::from_utf8(vec![240, 159, 146, 150]));

    let lossy: [&[u8]; 7] = [
        b"fine",
        b"a\xffb\xe2\x82c\xf0\x9f\x98",
        b"\xed\xa0\x80z",
        b"\xf4\x90\x80\x80!",
        b"\xe0\x80\x80",
        b"\xf0\x80\x80\xc1",
        b"\xe1\x80\x41\xf1\x80\x80",
    ];
    for bytes in lossy {
        let text = String::from_utf8_lossy(bytes);
        let kind = match &text {
            Cow::Borrowed(_) => "borrowed",
            Cow::Owned(_) => "owned",
        };
        println!("{} {:?} {}", kind, text, text.len());
    }
    let owned: String = String::from_utf8_lossy(b"x\x80").into_owned();
    println!("{}", owned);

    // SAFETY: the euro sign's bytes.
    let euro = unsafe { std::str::from_utf8_unchecked(&[0xe2, 0x82, 0xac]) };
    println!("{} {}", euro, std::str::from_utf8(&inputs[2][2..3]).is_ok());
}
