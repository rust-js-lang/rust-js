// Generic code's values and `dyn`s written into a writer of the crate's
// own, or a `W: fmt::Write`'s, a piece at a time, as std's `write_fmt`
// does (#1652, step 5 of ADR 1135): each by its formatting dictionary's
// `fmtInto`, which writes the pieces std's `fmt` of it writes. A `T`'s, a
// generic type's own parameters', `Duo<T>`'s `T`, each radix trait's, one
// given on through another generic writer, a `dyn Display`'s and a `dyn
// Error`'s, std's message errors' too; and a writer that fails at its nth
// call, where what was written before stays. A case of its own, apart from
// test/writer-sink.rs, whose writers are framed only where a type is shown,
// which a `dyn` written here would make every writer.
use std::fmt::{self, Write};

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

#[derive(Debug)]
struct Duo<T>(T, T);

impl<T: fmt::Display> fmt::Display for Duo<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "({}, {:>3})", self.0, self.1)
    }
}

#[derive(Debug)]
struct Labelled<T> {
    label: &'static str,
    value: Option<T>,
}

struct Hexed(u32);

impl fmt::LowerHex for Hexed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "h{:x}", self.0)
    }
}

impl fmt::UpperHex for Hexed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.pad_integral(true, "0x", "AB")
    }
}

fn shown_generic<W: Write, T: fmt::Display + fmt::Debug>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "{}|{:>6}|{:?}|{:#?}|{:<4?}", t, t, t, t, t)
}

fn radixes<W: Write, T: fmt::LowerHex + fmt::UpperHex>(w: &mut W, t: T) -> fmt::Result {
    write!(w, "{:x}|{:#06x}|{:X}|{:>8X}", t, t, t, t)
}

fn bits<W: Write, T: fmt::Octal + fmt::Binary>(w: &mut W, t: T) -> fmt::Result {
    write!(w, "{:o}|{:#o}|{:b}|{:#010b}", t, t, t, t)
}

fn passed_on<W: Write, T: fmt::Display + ?Sized>(w: &mut W, t: &T) -> fmt::Result {
    shown_once(w, t)?;
    shown_once(w, t)
}

fn shown_once<W: Write, T: fmt::Display + ?Sized>(w: &mut W, t: &T) -> fmt::Result {
    write!(w, "[{:^5}]", t)
}

#[derive(Debug)]
struct Failure(u8);

impl fmt::Display for Failure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "failure {}", self.0)
    }
}

impl std::error::Error for Failure {}

fn failing(n: u8) -> Result<(), Box<dyn std::error::Error>> {
    Err(Failure(n))?
}

fn main() {
    let p = Point { x: -1, y: 2 };
    let mut l = Chars(Vec::new());
    shown_generic(&mut l, &-12i32).unwrap();
    shown_generic(&mut l, &"é\"").unwrap();
    shown_generic(&mut l, &2.5f64).unwrap();
    shown_generic(&mut l, &'q').unwrap();
    shown_generic(&mut l, &p).unwrap();
    shown_generic(&mut l, &Duo(-1, 20)).unwrap();
    passed_on(&mut l, &Duo(Name("ab"), Name("c"))).unwrap();
    println!("{:?}", l.0);
    let mut l = Log(Vec::new());
    write!(
        l,
        "{:?}|{:#?}|{:?}",
        Labelled {
            label: "n",
            value: Some(-3i64)
        },
        Labelled {
            label: "v",
            value: Some(vec![Some(1u8), None])
        },
        Labelled::<Point> {
            label: "p",
            value: None
        }
    )
    .unwrap();
    radixes(&mut l, 255u8).unwrap();
    radixes(&mut l, -2i16).unwrap();
    radixes(&mut l, Hexed(171)).unwrap();
    bits(&mut l, 9u32).unwrap();
    bits(&mut l, -1i8).unwrap();
    println!("{:?}", l.0);
    let mut l = Chars(Vec::new());
    passed_on(&mut l, &true).unwrap();
    passed_on(&mut l, "ab").unwrap();
    passed_on(&mut l, &Name("x")).unwrap();
    let shown: Vec<Box<dyn fmt::Display>> = vec![
        Box::new(7),
        Box::new("s"),
        Box::new(-2.5f32),
        Box::new(9u64),
        Box::new(Point { x: 8, y: -9 }),
        Box::new(Duo('a', 'b')),
    ];
    for d in &shown {
        write!(l, "{:>3};{}|", d, d).unwrap();
        passed_on(&mut l, &**d).unwrap();
    }
    println!("{:?}", l.0);
    let mut l = Chars(Vec::new());
    let message: Box<dyn std::error::Error> = "boom".into();
    let own = failing(4).unwrap_err();
    let io: Box<dyn std::error::Error> = Box::new(std::io::Error::other("oops"));
    let kind: Box<dyn std::error::Error> = Box::new(std::io::Error::from(std::io::ErrorKind::NotFound));
    let parse: Box<dyn std::error::Error> = Box::new("x".parse::<i32>().unwrap_err());
    for e in [&message, &own, &io, &kind, &parse] {
        write!(l, "{}|{:>8}|{:.2}|", e, e, e).unwrap();
    }
    println!("{:?}", l.0);
    for n in 1..8 {
        let mut nth = Nth(0, n, String::new());
        let r = shown_generic(&mut nth, &Duo(-5, 6));
        let mut dyn_nth = Nth(0, n, String::new());
        let d = write!(dyn_nth, "{:>4}", shown[3]);
        println!("{} {:?} {:?} | {:?} {:?}", n, nth.2, r, dyn_nth.2, d);
    }
}
