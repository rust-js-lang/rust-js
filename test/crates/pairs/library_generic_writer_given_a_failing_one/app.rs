//! Its writers' errors are the library's results: a file opened to read,
//! whose writes the OS refuses, and a writer of its own that refuses all.
use std::fs::File;
use std::io::{self, Write};

struct Refusing(u8);

impl Write for Refusing {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        match self.0 {
            0 => Err(io::Error::other("refused")),
            _ => Ok(bytes.len()),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub fn main() {
    let mut file = File::open("Cargo.toml").expect("the workspace's manifest");
    match dep::emit(&mut file) {
        Ok(()) => println!("written"),
        Err(e) => println!("refused: {e} {:?}", e.kind()),
    }
    println!("{} {}", dep::emitted(&mut file), dep::emit_or(&mut file));
    let mut refusing = Refusing(0);
    let mut taking = Refusing(1);
    println!("{:?}", dep::emit(&mut refusing).map_err(|e| e.to_string()));
    println!("{:?}", dep::emit_twice(&mut refusing).map_err(|e| e.to_string()));
    println!("{} {}", dep::emitted(&mut refusing), dep::emitted(&mut taking));
    println!("{}", dep::emit_ignoring(&mut refusing));
    println!("{} {}", dep::emit_or(&mut refusing), dep::emit_or(&mut taking));
    println!("{}", dep::emit_through(&mut refusing));
    let mut bytes: Vec<u8> = Vec::new();
    println!("{} {:?} {bytes:?}", dep::emitted(&mut bytes), dep::emit_twice(&mut bytes).is_ok());
}
