//@ run-fail: called `Result::unwrap()` on an `Err` value: Utf8Error { valid_up_to: 1, error_len: Some(1) }
// `unwrap()` of bytes that aren't UTF-8 panics with the error's `Debug`.

fn main() {
    let bytes = vec![b'a', 0xff];
    let text = std::str::from_utf8(&bytes).unwrap();
    println!("{}", text);
}
