// A writer's own `write_fmt` given an `fmt::Arguments` a function holds, a
// parameter's or a variable's, not made where it's given: it's given the
// function of the `Arguments`, as any of the crate's functions is, which
// its body writes a piece at a time by `fmt::write`, or only reads as text,
// `args.to_string()` (#1695).
use std::fmt::{self, Write};

struct Upper(String);

impl Write for Upper {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push_str(&s.to_uppercase());
        self.0.push('|');
        Ok(())
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.0.push('[');
        fmt::write(self, args)?;
        self.0.push(']');
        Ok(())
    }
}

fn relay(w: &mut Upper, args: fmt::Arguments<'_>) -> fmt::Result {
    w.write_fmt(args)
}

struct Texts(Vec<String>);

impl Write for Texts {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.0.push(format!("<{}>", args.to_string().len()));
        Ok(())
    }
}

fn texts(w: &mut Texts, args: fmt::Arguments<'_>) -> fmt::Result {
    w.write_fmt(args)
}

fn main() {
    let mut u = Upper(String::new());
    relay(&mut u, format_args!("a{}b", 1)).unwrap();
    let n = 7;
    let held = format_args!("x{n}y");
    u.write_fmt(held).unwrap();
    write!(u, "c{}d", 2).unwrap();
    println!("{}", u.0);
    let mut t = Texts(Vec::new());
    texts(&mut t, format_args!("abc{}", 12)).unwrap();
    let held = format_args!("{n}{n}");
    t.write_fmt(held).unwrap();
    println!("{:?}", t.0);
}
