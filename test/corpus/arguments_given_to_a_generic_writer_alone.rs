// #1745's program: a `format_args!` given to generic code that writes its
// `T` into a writer, the program's one write of an `fmt::Arguments` a
// piece at a time, which arms the sink it claims, each piece a write of
// the writer's.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

fn own<W: Write, T: fmt::Display>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "[{}]", t)
}

fn main() {
    let mut log = Log(Vec::new());
    let n = 1;
    own(&mut log, &format_args!("a{}", n)).unwrap();
    println!("{:?}", log.0);
}
