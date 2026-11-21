//! The library's error over an `io::Error`, written by its `Debug` and
//! `Display` into a writer of this crate's own: each piece a call of the
//! writer's, as natively (#1420).
use dep::read_failed;
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
    let e = read_failed(std::io::Error::new(std::io::ErrorKind::Other, "boom"));
    println!("{:?}", e);
    println!("{}", e);
    let mut c = Counted(String::new(), 0);
    write!(c, "{:?}", e).unwrap();
    println!("{} {}", c.0, c.1);
    let e = read_failed(std::io::Error::from(std::io::ErrorKind::NotFound));
    println!("{:#?}", e);
}
