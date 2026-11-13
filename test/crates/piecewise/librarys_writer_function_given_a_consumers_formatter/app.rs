//! Writes the library's list into a writer by the library's writer
//! function, given its own writer's `Formatter`, a piece at a time.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

struct Mine(dep::Listed);

impl fmt::Display for Mine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<")?;
        self.0.joined(f, "|")?;
        f.write_str(">")
    }
}

pub fn main() {
    let mut log = Log(Vec::new());
    write!(log, "{}", Mine(dep::Listed(vec![1, 20, 3]))).unwrap();
    println!("{:?} {}", log.0, Mine(dep::Listed(vec![4])));
}
