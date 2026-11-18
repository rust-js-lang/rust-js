// A conditional's value, pushed onto a buffer, is held in a temporary
// named after the call its last branch makes, an imported function's,
// `readPartial`: the import keeps its name, and the temporary takes
// another, as React's Flight server reads a debug channel's chunks
// (`readPartialStringChunk`, rd582). A temporary never takes a name an
// import of its module is given, nor the one it's reclaimed to (ADR 0569).
use std::cell::RefCell;

mod chunks {
    pub fn read_final(left: &str) -> String {
        format!("{left}.")
    }

    pub fn read_partial(left: &str, bytes: &[u8]) -> String {
        let mut text = left.to_string();
        for b in bytes {
            text.push(*b as char);
        }
        text
    }
}

use chunks::{read_final, read_partial};

fn progress(buffer: &RefCell<String>, done: bool, bytes: &[u8]) {
    buffer
        .borrow_mut()
        .push_str(&if done { read_final("end") } else { read_partial("", bytes) });
}

fn read_partial_twice(bytes: &[u8]) -> String {
    let read_partial = read_partial("x", bytes);
    format!("{read_partial}{}", read_partial.len())
}

fn main() {
    let buffer = RefCell::new(String::new());
    progress(&buffer, false, b"ab");
    progress(&buffer, false, b"cd");
    progress(&buffer, true, b"");
    println!("{}", buffer.borrow());
    println!("{}", read_partial_twice(b"yz"));
}
