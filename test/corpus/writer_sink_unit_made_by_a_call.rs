// A writer written a piece at a time whose `()` is made by a call: the
// call runs, its print first, then the `()` is written (#1756).
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

fn give() -> &'static () {
    println!("give ran");
    &()
}

struct Shown;

impl fmt::Debug for Shown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(give(), f)
    }
}

fn main() {
    let mut log = Log(Vec::new());
    write!(log, "[{:>4?}] [{:?}]", Shown, Shown).unwrap();
    println!("{:?}", log.0);
    let given = format!("[{:>4?}]", Shown);
    println!("{given}");
}
