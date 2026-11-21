//! The library's `ThreadRng`, shown, and written into a writer of this
//! crate's own, each piece a call of the writer's, as natively (#1420).
use dep::{failed, thread_rng};
use std::fmt::Write;

struct Counted(String, u32);

impl Write for Counted {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        self.1 += 1;
        self.0.push_str(s);
        Ok(())
    }
}

pub fn main() {
    let rng = thread_rng();
    println!("{:?}", rng);
    let mut c = Counted(String::new(), 0);
    write!(c, "{:?}", rng).unwrap();
    println!("{} {}", c.0, c.1);
    let mut c = Counted(String::new(), 0);
    write!(c, "{:?}", failed("boom")).unwrap();
    println!("{} {}", c.0, c.1);
}
