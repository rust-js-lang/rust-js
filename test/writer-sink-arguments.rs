// An `Arguments` given on to what writes it into a writer is written a
// piece at a time, as std's `write_fmt` writes it, wherever it was made:
// by a function's parameter, given on through another, a trait's method
// through a `dyn`, a writer's own `write_fmt` and a `Formatter`'s; and
// called plainly, `to_string()`, `format!` or `println!`, it's its text
// (#1652, step 6 of ADR 1135). A writer's `Err` ends it where Rust's does,
// what was written before it kept, and a value that fails ends it too.
use std::fmt::{self, Write};

// Records each piece it's given.
struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
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

// Fails at its third call.
struct Failing(u32, String);

impl Write for Failing {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 += 1;
        if self.0 == 3 {
            return Err(fmt::Error);
        }
        self.1.push_str(s);
        Ok(())
    }
}

// A writer that passes what it's given on to the writer it holds, bracketed,
// by `fmt::write`.
struct Bracketed<'a>(&'a mut Log);

impl Write for Bracketed<'_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_str(s)
    }

    fn write_fmt(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.0.write_str("<")?;
        fmt::write(self.0, args)?;
        self.0.write_str(">")
    }
}

// A value that fails, and one that shows a `char`.
struct Refused;

impl fmt::Display for Refused {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Err(fmt::Error)
    }
}

struct Point(i32, i32);

// Says each time it's shown, which a writer runs once per write.
struct Loud(u8);

impl fmt::Display for Loud {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        println!("shown {}", self.0);
        write!(f, "{}!", self.0)
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.0, self.1)
    }
}

fn log_to(w: &mut Log, args: fmt::Arguments<'_>) -> fmt::Result {
    w.write_fmt(args)
}

fn relay(w: &mut Log, args: fmt::Arguments<'_>) {
    log_to(w, args).unwrap();
    log_to(w, args).unwrap();
}

fn into_any<W: Write>(w: &mut W, args: fmt::Arguments<'_>) -> fmt::Result {
    w.write_fmt(args)
}

fn into_dyn(w: &mut dyn Write, args: fmt::Arguments<'_>) -> fmt::Result {
    fmt::write(w, args)
}

// Writes what it's given, and its text.
fn both(w: &mut Log, args: fmt::Arguments<'_>) -> String {
    w.write_fmt(args).unwrap();
    println!("both: {args}");
    format!("[{}|{}]", args, args.to_string().len())
}

trait Sink {
    fn put(&mut self, args: fmt::Arguments<'_>) -> fmt::Result;
    fn text(&self) -> String;
}

impl Sink for Log {
    fn put(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.write_fmt(args)
    }

    fn text(&self) -> String {
        format!("{:?}", self.0)
    }
}

impl Sink for String {
    fn put(&mut self, args: fmt::Arguments<'_>) -> fmt::Result {
        self.push_str(&args.to_string());
        Ok(())
    }

    fn text(&self) -> String {
        format!("{self:?}")
    }
}

// A `Formatter` given an `Arguments`.
struct Prefixed<'a>(&'a str, i32);

fn prefixed(f: &mut fmt::Formatter<'_>, args: fmt::Arguments<'_>) -> fmt::Result {
    f.write_str("~")?;
    f.write_fmt(args)
}

impl fmt::Display for Prefixed<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        prefixed(f, format_args!("{}{:>3}", self.0, self.1))
    }
}

fn main() {
    let n = -5;
    let mut log = Log(Vec::new());
    log_to(&mut log, format_args!("a{}b{:>4}", n, "x")).unwrap();
    relay(&mut log, format_args!("[{:?}]", "q\"r"));
    log_to(&mut log, format_args!("plain")).unwrap();
    println!("{:?}", log.0);

    let mut chars = Chars(Vec::new());
    into_any(&mut chars, format_args!("{}|{:^5}|{}", 'é', 'c', Point(1, -2))).unwrap();
    into_dyn(&mut chars, format_args!("{:+}{}", 3, 'z')).unwrap();
    let mut text = String::new();
    into_any(&mut text, format_args!("{}-{}", n, Point(3, 4))).unwrap();
    into_dyn(&mut text, format_args!("{:05}", n)).unwrap();
    into_any(&mut text, format_args!("<{}>", Loud(1))).unwrap();
    into_dyn(&mut text, format_args!("<{}>", Loud(2))).unwrap();
    println!("{:?} {:?}", chars.0, text);

    let mut failing = Failing(0, String::new());
    let r = into_any(&mut failing, format_args!("{}{}{n}{}", 'a', "b", 'd'));
    println!("{:?} {} {:?}", failing.1, failing.0, r);
    let mut refused = Log(Vec::new());
    let r = log_to(&mut refused, format_args!("x{}y{}", 1, Refused));
    println!("{:?} {:?}", refused.0, r);
    let mut s = String::new();
    let r = into_any(&mut s, format_args!("x{}y{}", 1, Refused));
    println!("{s:?} {r:?}");

    let mut both_log = Log(Vec::new());
    let made = both(&mut both_log, format_args!("{}={:<3}|", "k", n));
    println!("{:?} {}", both_log.0, made);

    let mut sinks: Vec<Box<dyn Sink>> = vec![Box::new(Log(Vec::new())), Box::new(String::new())];
    for (i, sink) in sinks.iter_mut().enumerate() {
        sink.put(format_args!("{i}:{}", Point(i as i32, -1))).unwrap();
        println!("{}", sink.text());
    }
    let mut own = Log(Vec::new());
    own.put(format_args!("{}{}", 'p', 7)).unwrap();
    println!("{:?}", own.0);

    let mut inner = Log(Vec::new());
    let mut bracketed = Bracketed(&mut inner);
    write!(bracketed, "{}+{}", 1, -2).unwrap();
    into_any(&mut bracketed, format_args!("{:>3}", 'q')).unwrap();
    println!("{:?}", inner.0);

    let mut shown = Chars(Vec::new());
    write!(shown, "{}|{:>9}", Prefixed("ab", -7), Prefixed("é", 8)).unwrap();
    println!("{:?} {}", shown.0, Prefixed("x", 1));
}
