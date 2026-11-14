//@ library-differs
// A `format_args!` given to generic code that writes its `T` into a writer
// through its dictionary, `own(w, &format_args!(..))`: the function that
// writes into the sink it claims, each piece a write of the writer's, as
// std writes them (#1745). By reference and by value, held in a variable,
// in an array a `&[T]` is given, and in a struct whose `Display` writes
// it; one read for its text is that text, one write.
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

struct Wrapper<T>(T);

impl<T: fmt::Display> fmt::Display for Wrapper<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({})", self.0)
    }
}

fn own<W: Write, T: fmt::Display>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "[{}]", t)
}

fn owned<W: Write, T: fmt::Display>(w: &mut W, t: T) -> fmt::Result {
    write!(w, "<{}>", t)
}

fn each<W: Write, T: fmt::Display>(w: &mut W, items: &[T]) -> fmt::Result {
    for item in items {
        write!(w, "{};", item)?;
    }
    Ok(())
}

fn main() {
    let mut log = Log(Vec::new());
    let n = 1;
    let s = "b";
    own(&mut log, &format_args!("a{}", n)).unwrap();
    owned(&mut log, format_args!("{s}{n}{}", n + 1)).unwrap();
    let args = format_args!("c{}", n);
    own(&mut log, &args).unwrap();
    let text = format_args!("d{}e", n).to_string();
    own(&mut log, &text).unwrap();
    each(&mut log, &[format_args!("x{}", n), format_args!("y")]).unwrap();
    own(&mut log, &Wrapper(format_args!("w{}", n))).unwrap();
    let pair = (format_args!("p{}", n), 2);
    own(&mut log, &pair.0).unwrap();
    println!("{:?}", log.0);
    println!("{}", Wrapper(format_args!("plain{}", n)));
}
