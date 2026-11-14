//@ library-differs
// writer_sink_generic_instances' generic type, whose writer reads its
// `Formatter`'s options, so the crate's writers take them: each instance
// asked for gives its parameters dictionaries that write by those options
// (#1738), and a dictionary's `fmtInto` gives its writer the options before
// the dictionaries it's given (#1739).
use std::fmt::{self, Write};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

struct Wrap<T>(T);

impl<T: fmt::Display> fmt::Display for Wrap<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if f.alternate() {
            write!(f, "<<{}>>", self.0)
        } else {
            write!(f, "<{}>", self.0)
        }
    }
}

struct Tag(&'static str);

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad(self.0)
    }
}

fn wrapped<T: fmt::Display>(t: T) -> String {
    format!("{:#}", Wrap(t))
}

fn main() {
    let mut log = Log(Vec::new());
    write!(log, "[{:#}]", Wrap(Wrap(2.5))).unwrap();
    write!(log, "[{}]", Wrap(Tag("t"))).unwrap();
    println!("{:?}", log.0);
    println!("{}", Wrap(String::from("s")));
    println!("{:#}", Wrap(Wrap(Tag("x"))));
    println!("{}", wrapped(9u16));
    println!("{}", wrapped(Tag("y")));
    let shown: Box<dyn fmt::Display> = Box::new(Wrap('z'));
    println!("{shown:#}");
}
