// A `write!` into a writer of the crate's own, or a `W: fmt::Write`'s, a
// `dyn`'s too, gives its `write_str` a piece at a time, as std's
// `write_fmt` does: each literal piece, each string a value writes, an
// integer's sign apart, a `write_char`, and each fill char a `write_char`,
// the writer's own where it has one (#1652, ADR 1135). A writer that
// counts its calls, or fails at one, sees what it sees in Rust: what was
// written before its `Err` stays, and nothing after it is written. A
// literal argument rustc writes into the format string, `b{}` of `1`, is
// one piece, as Rust's is.
use std::fmt::{self, Write};
use std::panic::{catch_unwind, AssertUnwindSafe};

struct Log(Vec<String>);

impl Write for Log {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.push(s.to_string());
        Ok(())
    }
}

// A writer with its own `write_char`, which marks each char it's given.
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

// Fails on the third call.
struct Third(u32, String);

impl Write for Third {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 += 1;
        if self.0 == 3 {
            return Err(fmt::Error);
        }
        self.1.push_str(s);
        Ok(())
    }
}

fn into_any<W: Write>(w: &mut W, n: i32, s: &str, c: char) -> fmt::Result {
    write!(w, "[{:>4}|{:<3}|{:^5}|{:+}|{:05}]", n, s, c, n, n)?;
    writeln!(w, "{:#x} {:x} {:#010b} {:o}", n, n, n, n)
}

fn into_dyn(w: &mut dyn Write, b: bool, s: &str) -> fmt::Result {
    write!(w, "{} {:>6} {:.2} {:*^7.1}|", b, b, s, s)
}

// The issue's: each call counted, and the piece holding an `x` fails.
struct Counted {
    text: String,
    calls: u32,
}

impl Write for Counted {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.calls += 1;
        if s.contains('x') {
            return Err(fmt::Error);
        }
        self.text.push_str(s);
        Ok(())
    }
}

fn main() {
    let args: Vec<String> = ["ab", "cd", "a", "x"].iter().map(|s| s.to_string()).collect();
    let mut counted = Counted {
        text: String::new(),
        calls: 0,
    };
    let r = write!(counted, "{}-{}", args[0], args[1]);
    println!("{:?} {} {:?}", counted.text, counted.calls, r);
    let r = write!(counted, "[{}{}]", args[2], args[3]);
    println!("{:?} {} {:?}", counted.text, counted.calls, r);
    let mut inlined = Log(Vec::new());
    write!(inlined, "b{}", 1).unwrap();
    writeln!(inlined, "{}", args[0]).unwrap();
    writeln!(inlined).unwrap();
    fmt::write(&mut inlined, format_args!("{}|{}", args[1], -3)).unwrap();
    println!("{:?}", inlined.0);

    let nums: Vec<i32> = vec![-42, 7, 0, i32::MIN];
    let big: Vec<u64> = vec![u64::MAX, 3];
    let wide: Vec<i128> = vec![-170141183460469231731687303715884105728];
    let s = String::from("héllo");
    let c = 'é';
    let b = nums[2] == 0;

    let mut log = Log(Vec::new());
    write!(log, "{} {} {} {} {}", nums[0], nums[3], big[0], big[1], wide[0]).unwrap();
    write!(log, "{:#} {:#?} {:?} {:?}", nums[1], nums[1], nums[0], b).unwrap();
    write!(log, "{:x?} {:#X?}", nums[0], big[1]).unwrap();
    write!(log, "{}{}{}", c, s, c).unwrap();
    write!(log, "{:3}|{:<3}|{:>3}", c, c, c).unwrap();
    write!(log, "{:.0}|{:.1}|{:10.3}|", s, s, s).unwrap();
    write!(log, "{:wd$}|{:.*}|{:>w$.p$}", nums[0], 2, s, s, wd = 6, w = 5, p = 1).unwrap();
    println!("{:?}", log.0);

    let mut chars = Chars(Vec::new());
    write!(chars, "{:*>5}|{:-^6}|{}|{:+}", nums[1], s, c, nums[1]).unwrap();
    write!(chars, "{:08.3}|{:<+5}", nums[0], nums[1]).unwrap();
    println!("{:?}", chars.0);

    let mut any = Log(Vec::new());
    into_any(&mut any, nums[0], &s, c).unwrap();
    into_any(&mut any, nums[1], "", 'x').unwrap();
    println!("{:?}", any.0);
    let mut text = String::new();
    into_any(&mut text, nums[0], &s, c).unwrap();
    println!("{:?}", text);
    let mut own = Chars(Vec::new());
    into_any(&mut own, nums[1], &s, c).unwrap();
    println!("{:?}", own.0);

    let mut d = Log(Vec::new());
    into_dyn(&mut d, b, &s).unwrap();
    into_dyn(&mut d, !b, "").unwrap();
    println!("{:?}", d.0);

    for n in &nums {
        let mut t = Third(0, String::new());
        let r = write!(t, "{:>3}{}", n, s);
        println!("{} {:?} {:?}", t.0, t.1, r);
    }
    let mut t = Third(0, String::new());
    let r = into_any(&mut t, nums[1], &s, c);
    println!("{} {:?} {:?}", t.0, t.1, r);

    // Each argument is evaluated before anything is written: the index
    // panics with nothing written.
    let mut early = Log(Vec::new());
    let at = nums.len() + 1;
    let ended = catch_unwind(AssertUnwindSafe(|| write!(early, "{}{}", s, nums[at])));
    println!("{} {:?}", ended.is_err(), early.0);
}
