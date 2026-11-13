// A `write!` through generic code's `W: fmt::Write` calls `W::write_fmt`,
// as Rust does: a writer's own `write_fmt` where it defines one, std's
// otherwise, which writes each piece into its `write_str` (#1652, step 6
// of ADR 1135). So does a `&mut W`, which std's impl passes on to `W`'s,
// an `Either`-like writer whose own `write_fmt` passes its `Arguments` on
// to the side it holds, and a `Formatter` given as a `W`.
use std::fmt::{self, Write};

// Records each piece it's given.
struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

// Counts its `write_fmt` calls, and writes the text it's given, bracketed.
struct Counting {
    calls: u32,
    text: String,
}

impl Write for Counting {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.text.push_str(s);
        Ok(())
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.calls += 1;
        self.text.push('<');
        self.text.push_str(&args.to_string());
        self.text.push('>');
        Ok(())
    }
}

// Either side, its own `write_fmt` passing what it's given on, as the
// `either` crate's does.
enum Either<L, R> {
    Left(L),
    Right(R),
}

impl<L: Write, R: Write> Write for Either<L, R> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        match self {
            Either::Left(inner) => inner.write_str(s),
            Either::Right(inner) => inner.write_str(s),
        }
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        match self {
            Either::Left(inner) => inner.write_fmt(args),
            Either::Right(inner) => inner.write_fmt(args),
        }
    }
}

// Counts its `write_fmt` calls, and writes what it's given into itself by
// `fmt::write`, std's own `write_fmt`, which writes each piece through its
// `write_str`, never its `write_fmt` (#1697).
struct Counted(Vec<String>, u32);

impl Write for Counted {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.1 += 1;
        fmt::write(self, args)
    }
}

// Shown only by a `format_args!` given a function that writes it into a
// writer: its writer writes each piece into that writer.
struct Wrapped(i32);

impl fmt::Display for Wrapped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}>", self.0)
    }
}

fn logged(w: &mut Log, args: fmt::Arguments<'_>) -> fmt::Result {
    w.write_fmt(args)
}

fn put<W: Write>(w: &mut W, n: i32) -> fmt::Result {
    write!(w, "[{n:>3}]")
}

// A `Formatter` given as a `W`.
struct Shown(i32);

impl fmt::Display for Shown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        put(f, self.0)
    }
}

// Marks each char it's given by its own `write_char`.
struct Chars(Vec<String>);

impl Write for Chars {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }

    fn write_char(&mut self, c: char) -> fmt::Result {
        self.0.push(format!("<{c}>"));
        Ok(())
    }
}

fn put_char<W: Write>(w: &mut W, c: char) -> fmt::Result {
    w.write_char(c)?;
    w.write_str("|")
}

// A `Formatter` given as a `W`, written a `char`: the writer's own
// `write_char` it's shown into.
struct Marked(char);

impl fmt::Display for Marked {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        put_char(f, self.0)
    }
}

fn main() {
    let mut counting = Counting {
        calls: 0,
        text: String::new(),
    };
    put(&mut counting, 7).unwrap();
    put(&mut &mut counting, -8).unwrap();
    write!(counting, "{}.", 6).unwrap();
    println!("{} {:?}", counting.calls, counting.text);

    let mut log = Log(Vec::new());
    put(&mut log, 7).unwrap();
    put(&mut &mut log, -8).unwrap();
    logged(&mut log, format_args!("{}", Wrapped(-3))).unwrap();
    println!("{:?}", log.0);

    let mut left: Either<Log, Counting> = Either::Left(Log(Vec::new()));
    put(&mut left, 1).unwrap();
    let mut right: Either<Log, Counting> = Either::Right(Counting {
        calls: 0,
        text: String::new(),
    });
    put(&mut right, 2).unwrap();
    write!(right, "{}!", 3).unwrap();
    match (left, right) {
        (Either::Left(log), Either::Right(counting)) => {
            println!("{:?} {} {:?}", log.0, counting.calls, counting.text)
        }
        _ => unreachable!(),
    }

    let mut text = String::new();
    put(&mut text, 9).unwrap();
    println!("{text:?} {}", Shown(-4));

    let mut shown = Log(Vec::new());
    write!(shown, "{}|{:>8}", Shown(5), Shown(-6)).unwrap();
    println!("{:?}", shown.0);

    let mut chars = Chars(Vec::new());
    write!(chars, "{}{}", Marked('é'), Shown(1)).unwrap();
    println!("{:?} {}", chars.0, Marked('x'));

    let mut counted = Counted(Vec::new(), 0);
    let n = -4;
    write!(counted, "[{n:>3}]").unwrap();
    put(&mut counted, 5).unwrap();
    fmt::write(&mut counted, format_args!("<{}>", n)).unwrap();
    println!("{:?} {}", counted.0, counted.1);
}
