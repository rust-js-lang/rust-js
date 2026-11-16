//! A consumer's engine, given to the library's copies of each.
use dep::Engine;

struct Padded(u32);

impl Engine for Padded {
    fn pad(&self) -> bool {
        self.0 > 0
    }
}

impl Drop for Padded {
    fn drop(&mut self) {
        println!("dropped {}", self.0);
    }
}

pub fn main() {
    let mut out = String::new();
    Padded(1).encode_string(b"hello world", &mut out);
    println!("{out}");
    let plain = Padded(0);
    println!("{}", dep::Display::new(b"abcdefg", &plain));
    println!("{}", dep::Display::new(b"abcdefg", &Padded(2)));
}
