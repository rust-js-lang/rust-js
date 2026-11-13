//@ library-differs
// A `write!` into a writer of the crate's own, or a `W: fmt::Write`'s, a
// `dyn`'s too, gives its `write_str` a piece at a time, as std's
// `write_fmt` does: each literal piece, each string a value writes, an
// integer's sign apart, a `write_char`, and each fill char a `write_char`,
// the writer's own where it has one (#1652, ADR 1135). A writer that
// counts its calls, or fails at one, sees what it sees in Rust: what was
// written before its `Err` stays, and nothing after it is written. A
// literal argument rustc writes into the format string, `b{}` of `1`, is
// one piece, as Rust's is.
//
// A writer of the crate's own shown into one writes into that writer too,
// a piece at a time, as std's `Formatter` does: its own `write!`, each
// `write_str` and `write_char`, `pad` and `pad_integral` by its options, a
// `fmt` it gives its `Formatter` and a writer function it gives it to. A
// `format!`, a `to_string()` or a `write!` into a `String` it makes is a
// string of its own, written where the writer writes it. A writer's own
// `Err`, and the writer's, end the `write!` where Rust's does.
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

// Fails on the nth call.
struct Nth(u32, u32, String);

impl Write for Nth {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0 += 1;
        if self.0 == self.1 {
            return Err(fmt::Error);
        }
        self.2.push_str(s);
        Ok(())
    }
}

struct Point {
    x: i32,
    y: i32,
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {})", self.x, self.y)
    }
}

impl fmt::Debug for Point {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("P")?;
        fmt::Display::fmt(&self.x, f)?;
        f.write_char(';')?;
        fmt::Debug::fmt(&self.y, f)
    }
}

struct Name(&'static str);

impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_char('<')?;
        f.pad(self.0)?;
        f.write_str(">")
    }
}

// Gives its `Formatter`, and its options, to std's `char`, `bool` and `str`.
struct Letter(char, bool, &'static str);

impl fmt::Display for Letter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)?;
        fmt::Display::fmt(&self.1, f)?;
        fmt::Debug::fmt(&self.1, f)?;
        fmt::Display::fmt(self.2, f)
    }
}

// Formats its inner value into a `String` first, three ways, then writes
// the text: each made apart, then written whole, one call.
struct Staged(Point);

impl fmt::Display for Staged {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let inner = format!("{}", self.0);
        let again = self.0.to_string();
        let mut built = String::new();
        write!(built, "{}|{}", self.0, Name("n"))?;
        write!(f, "{inner}/{again}/{built}")
    }
}

// Makes its inner value's text before it writes anything.
struct Early(Point);

impl fmt::Display for Early {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = self.0.to_string();
        f.write_str(&text)
    }
}

struct Wrap(Point);

impl fmt::Display for Wrap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}]", self.0)?;
        fmt::Display::fmt(&self.0.x, f)?;
        fmt::Display::fmt(&self.0, f)
    }
}

// Gives its `Formatter`, and its options, to std's floats.
struct Celsius(f64, f32);

impl fmt::Display for Celsius {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)?;
        f.write_str("C/")?;
        fmt::Debug::fmt(&self.1, f)
    }
}

// A `Debug` of the crate's own by each of std's builders, `{:#?}`'s through
// std's `PadAdapter`.
struct Unit;

impl fmt::Debug for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Unit")
    }
}

struct Manual(Vec<u8>);

impl fmt::Debug for Manual {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Manual").field("a", &1).field("b", &"two").finish()?;
        f.debug_tuple("T").field(&3.5).field(&Unit).finish()?;
        f.debug_list().entries(self.0.iter()).entry(&-1).finish()?;
        f.debug_map().entry(&"k", &'v').entries([(1, "x")]).finish()?;
        f.debug_set().entry(&Unit).finish()?;
        f.debug_struct("More").field("c", &None::<u8>).finish_non_exhaustive()?;
        f.debug_struct("Empty").finish()?;
        f.debug_tuple("").field(&()).finish()
    }
}

// Derived, of std's composites and of the crate's own.
#[derive(Debug)]
struct Holder {
    m: Manual,
    tags: Vec<&'static str>,
    pair: (i8, Option<char>),
}

#[derive(Debug)]
struct Wrapped(u8, char);

#[derive(Debug)]
enum Shape {
    Dot,
    Line(i32),
    Box { sides: Vec<u8> },
}

// Fails at its builder's second field, where the writer fails.
#[derive(Debug)]
struct Fields {
    first: i32,
    second: &'static str,
}

// Prints as it formats, so it writes through a sink, but is never shown
// into a writer: its text is made as before, a generic part and all.
struct Echo<T>(T);

impl<T: fmt::Display> fmt::Display for Echo<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        println!("echo");
        write!(f, "({})", self.0)
    }
}

// Shown only by `Branch`'s writer, which is shown into a writer.
struct Leaf(i8);

impl fmt::Display for Leaf {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{{}}}", self.0)
    }
}

struct Branch(Leaf);

impl fmt::Display for Branch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "b{}", self.0)
    }
}

// Keeps each write's result and carries on.
struct Stubborn(i32);

impl fmt::Display for Stubborn {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let first = f.write_str("a");
        let second = write!(f, "{}", self.0);
        if first.is_err() || second.is_err() {
            f.write_str("!")?;
        }
        f.write_str("z")
    }
}

// Fails of its own, after writing.
struct Refuses;

impl fmt::Display for Refuses {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("r")?;
        Err(fmt::Error)
    }
}

struct Nothing;

impl fmt::Display for Nothing {
    fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result {
        Ok(())
    }
}

fn bracketed(f: &mut fmt::Formatter<'_>, n: i32) -> fmt::Result {
    write!(f, "<{}>", n)
}

// Gives its `Formatter` to a function, and prints as it formats.
struct Helped(i32);

impl fmt::Display for Helped {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        bracketed(f, self.0)?;
        println!("inside Helped");
        f.write_str("!")
    }
}

fn shown<W: Write>(w: &mut W, p: &Point) -> fmt::Result {
    write!(w, "{}{:?}", p, p)
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
    println!("{} {}", Echo(1), Echo("two"));

    let p = Point { x: -1, y: 2 };
    let mut framed = Log(Vec::new());
    write!(framed, "a{}b{:?}", p, p).unwrap();
    write!(framed, "{:x?}|{:#X?}|{:5?}", p, p, p).unwrap();
    write!(
        framed,
        "{:>6}|{}|{:<4}|{:^5}",
        Name("hi"),
        Name("x"),
        Name("é"),
        Name("ab")
    )
    .unwrap();
    write!(framed, "{}", Wrap(Point { x: 3, y: -4 })).unwrap();
    write!(framed, "{:+}", Wrap(Point { x: 3, y: -4 })).unwrap();
    write!(framed, "{}", Staged(Point { x: 5, y: 6 })).unwrap();
    write!(framed, "{}{}", Early(Point { x: 7, y: -8 }), Nothing).unwrap();
    write!(framed, "<{}>{}", Stubborn(-9), Helped(-1)).unwrap();
    write!(framed, "{}", Branch(Leaf(-7))).unwrap();
    println!("{:?}", framed.0);

    let mut chars = Chars(Vec::new());
    write!(chars, "{:*>4}{}", Name("c"), p).unwrap();
    write!(
        chars,
        "{}|{:>3}|{:-<2.1}",
        Letter('é', true, "ab"),
        Letter('x', false, "cd"),
        Letter('y', true, "ef")
    )
    .unwrap();
    shown(&mut chars, &p).unwrap();
    println!("{:?}", chars.0);

    for n in 1..9 {
        let mut nth = Nth(0, n, String::new());
        let r = write!(nth, "{}", Wrap(Point { x: 1, y: -2 }));
        let mut stubborn = Nth(0, n, String::new());
        let s = write!(stubborn, "{}", Stubborn(4));
        println!("{} {:?} {:?} | {:?} {:?}", n, nth.2, r, stubborn.2, s);
    }
    let mut refused = Log(Vec::new());
    let r = write!(refused, "a{}b", Refuses);
    println!("{:?} {:?}", refused.0, r);

    // Floats, by `flt2dec`'s parts: the sign a `write_str`, an empty one
    // where `{:0}` writes it first, zeros by at most 64, each fill char a
    // `write_char`, and exact digits only as many as std makes.
    let floats: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.5,
        0.25,
        100.0,
        120.5,
        1e16,
        1e100,
        1e-7,
        0.0001,
        f64::NAN,
        f64::NEG_INFINITY,
        0.096,
        f64::MAX,
        5e-324,
        2.5,
    ];
    for x in &floats {
        let mut l = Log(Vec::new());
        write!(
            l,
            "{}|{:?}|{:.2}|{:.0}|{:e}|{:E}|{:.3e}|{:+}|{:8.1}|{:08.2}|{:<7}|{:.30}|{:.0e}|{:^9?}|{:+010.1e}",
            x, x, x, x, x, x, x, x, x, x, x, x, x, x, x
        )
        .unwrap();
        println!("{} {:?}", x, l.0);
    }
    let singles: Vec<f32> = vec![0.1, 0.125, 1.0, -2.5e-8, 3.4e38, f32::MIN_POSITIVE, 1e-45, 16777217.0];
    for y in &singles {
        let mut l = Chars(Vec::new());
        write!(l, "{}|{:?}|{:.3}|{:e}|{:.60}|{:>12.2e}", y, y, y, y, y, y).unwrap();
        println!("{} {:?}", y, l.0);
    }
    // A string's `Debug`: its quote a `write_char`, each run of `char`s as
    // they are a `write_str`, each escape one, the rest one, `""` too; a
    // `char`'s, its quotes and itself `write_char`s, or its escape. Neither
    // pads to a width.
    let texts = ["a\"b\nc", "", "plain", "é\u{301}x\u{7f}y'z\\", "\u{200b}\u{10ffff}"];
    for s in texts {
        let mut l = Chars(Vec::new());
        write!(l, "{:?}|{:10?}|{:<12?}|{:?}", s, s, s, String::from(s)).unwrap();
        println!("{:?}", l.0);
    }
    for c in ['a', '\'', '"', '\n', '\u{301}', 'é', '\0', '\u{ad}'] {
        let mut l = Chars(Vec::new());
        write!(l, "{:?}|{:5?}|{:x?}", c, c, c).unwrap();
        println!("{:?}", l.0);
    }
    // std's composites and builders, each piece as core's builders write
    // it, `{:#?}`'s through std's `PadAdapter`, its options given to each
    // leaf, a derived `Debug`'s and one of the crate's own builders.
    let composites = Holder {
        m: Manual(vec![1]),
        tags: vec!["x", "y\n"],
        pair: (-2, Some('\n')),
    };
    let nested = vec![vec![1u8, 2], vec![]];
    let some: Option<i32> = Some(-1);
    let none: Option<&str> = None;
    let ok: Result<u8, String> = Ok(3);
    let err: Result<u8, String> = Err("bad".to_string());
    let options = [Some('a'), None];
    let mut l = Chars(Vec::new());
    write!(
        l,
        "{:?}|{:#?}|{:?}|{:#?}|{:?}|{:?}",
        nested, nested, some, none, ok, err
    )
    .unwrap();
    write!(
        l,
        "{:?}|{:#?}|{:?}|{:#?}|{:5?}|{:#x?}",
        (1, "x"),
        (2.5f32,),
        options,
        options,
        nested,
        nested
    )
    .unwrap();
    write!(l, "{:?}|{:08.2?}|{:?}", (), (1.5, -2.25), Box::new(some)).unwrap();
    println!("{:?}", l.0);
    let mut l = Log(Vec::new());
    write!(l, "{:?}|{:#?}|{:4?}", composites, composites, composites).unwrap();
    write!(l, "{:?}|{:#?}", Manual(vec![7, 8]), Manual(vec![])).unwrap();
    println!("{:?}", l.0);
    let mut l = Chars(Vec::new());
    write!(
        l,
        "{:?}|{:?}|{:?}|{:#?}|{:?}",
        Wrapped(1, 'c'),
        Shape::Dot,
        Shape::Line(-3),
        Shape::Box { sides: vec![1, 2] },
        [Shape::Dot]
    )
    .unwrap();
    println!("{:?}", l.0);
    for n in 1..12 {
        let mut nth = Nth(0, n, String::new());
        let r = write!(nth, "{:?}", Fields { first: -1, second: "s" });
        let mut pretty = Nth(0, n, String::new());
        let p = write!(pretty, "{:#?}", vec![Some(1)]);
        println!("{} {:?} {:?} | {:?} {:?}", n, nth.2, r, pretty.2, p);
    }

    let mut celsius = Chars(Vec::new());
    write!(
        celsius,
        "{}|{:>9.2}|{:+}|{:07.1}",
        Celsius(-3.25, 1e-5),
        Celsius(21.0, 0.5),
        Celsius(0.0, -0.0),
        Celsius(1.05, 2.0)
    )
    .unwrap();
    println!("{:?}", celsius.0);

    println!(
        "{} {:?} {:>5} {} {}",
        p,
        p,
        Name("q"),
        Wrap(Point { x: 0, y: 0 }),
        Staged(Point { x: 1, y: 1 })
    );
}
