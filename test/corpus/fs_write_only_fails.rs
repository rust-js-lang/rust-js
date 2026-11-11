// A crate whose only write that may fail is a `File`'s, given to generic
// code (#1156): the OS refuses a write to a file opened to read, and the
// error is generic code's value, as Rust's is, though nothing else of the
// crate fails. The file is the program's own, opened to read alone.
use std::fs::File;
use std::io::{self, Write};

fn emit<W: Write>(out: &mut W) -> io::Result<()> {
    out.write_all(b"x")
}

fn main() {
    let program = std::env::args().next().expect("the program's path");
    let mut file = File::open(program).expect("the program is there");
    match emit(&mut file) {
        Ok(()) => println!("written"),
        Err(e) => println!("refused: {e} {:?}", e.kind()),
    }
}
