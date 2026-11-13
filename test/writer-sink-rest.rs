// The rest of what std writes into a writer a piece at a time (#1652, ADR
// 1135): a map's and a set's `Debug`, by `debug_map` and `debug_set`; a
// derived `Debug` of more than five fields; an integer's `{:e}`, as core's
// `exp_u64` writes it; a closure given a writer's `Formatter`; and a
// `Formatter` a struct holds. Each writer
// records every call it's given, and one fails part way. A `dyn Debug`'s
// is test/writer-sink-generic.rs's, as a `dyn` written frames every writer.
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt::{self, Write};

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
struct Failing(Vec<String>);

impl Write for Failing {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        if self.0.len() == 2 {
            return Err(fmt::Error);
        }
        self.0.push(s.to_string());
        Ok(())
    }
}

struct Holder(BTreeMap<&'static str, Vec<i32>>);

impl fmt::Display for Holder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{:?}>", self.0)
    }
}

#[derive(Debug)]
struct Six {
    a: i32,
    b: &'static str,
    c: char,
    d: Option<u8>,
    e: Vec<i8>,
    f: bool,
}

#[derive(Debug)]
struct SixT(i32, &'static str, char, Option<u8>, Vec<i8>, bool, u8);

#[derive(Debug)]
enum Wide {
    Named { a: u8, b: u8, c: u8, d: u8, e: u8, f: u8 },
    Unnamed(u8, u8, u8, u8, u8, u8),
}

// A closure given the `Formatter`, called twice.
struct Closed(i32);

impl fmt::Display for Closed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let n = self.0;
        let w = |f: &mut fmt::Formatter<'_>| write!(f, "<{}>", n);
        w(f)?;
        f.write_str("|")?;
        w(f)
    }
}

// A `Formatter` held, as anyhow's `Quoted` holds it.
struct Quoted<'a, 'b>(&'a mut fmt::Formatter<'b>);

impl Write for Quoted<'_, '_> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_str(&s.replace('"', "\\\""))
    }
}

struct Held<'a, 'b>(&'a mut fmt::Formatter<'b>);

struct Shown(i32, &'static str);

impl fmt::Display for Shown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let held = Held(f);
        write!(held.0, "<{}>", self.0)?;
        held.0.write_char('|')?;
        let f = held.0;
        f.write_char('"')?;
        Quoted(f).write_fmt(format_args!("{}", self.1))?;
        f.write_char('"')
    }
}

// A `Cell` and a `RefCell`, a mutably borrowed one `<borrowed>`.
#[derive(Debug)]
struct Celled {
    count: Cell<i32>,
    names: RefCell<Vec<&'static str>>,
}

// Its own `LowerHex`, written by `{:x}`, and one that gives its
// `Formatter` on to a number's `LowerHex` and `LowerExp`, by its options.
struct Hexy(u32);

impl fmt::LowerHex for Hexy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("h")?;
        fmt::LowerHex::fmt(&self.0, f)
    }
}

impl fmt::LowerExp for Hexy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::LowerExp::fmt(&self.0, f)
    }
}

fn main() {
    let m = HashMap::from([("k", -1)]);
    let b = BTreeMap::from([(1, "a\"b"), (2, "c")]);
    let s = HashSet::from(['z']);
    let t = BTreeSet::from([3u8, 1, 2]);
    let empty: BTreeMap<u8, u8> = BTreeMap::new();
    let mut log = Log(Vec::new());
    write!(log, "{:?}|{:?}|{:?}|{:?}|{:?}", m, b, s, t, empty).unwrap();
    println!("{:?}", log.0);
    let mut log = Log(Vec::new());
    write!(log, "{:#?}|{:5?}|{:#?}", b, t, empty).unwrap();
    println!("{:?}", log.0);
    let h = Holder(BTreeMap::from([("x", vec![1, -2]), ("y", vec![])]));
    let mut log = Log(Vec::new());
    write!(log, "{}|{:#}", h, h).unwrap();
    println!("{:?}", log.0);

    let six = Six {
        a: -1,
        b: "x\"",
        c: 'é',
        d: Some(3),
        e: vec![-2],
        f: true,
    };
    let sixt = SixT(7, "y", 'z', None, vec![], false, 9);
    let mut log = Log(Vec::new());
    write!(log, "{:?}|{:#?}", six, sixt).unwrap();
    println!("{:?}", log.0);
    let mut log = Log(Vec::new());
    let named = Wide::Named {
        a: 1,
        b: 2,
        c: 3,
        d: 4,
        e: 5,
        f: 6,
    };
    write!(log, "{:?}|{:?}", named, Wide::Unnamed(1, 2, 3, 4, 5, 6)).unwrap();
    println!("{:?}", log.0);

    let (n, u, big, half) = (-1500i32, 120u8, 9_995_000_000i64, 2500u16);
    let mut chars = Chars(Vec::new());
    write!(
        chars,
        "{:e}|{:E}|{:.2e}|{:+e}|{:>10e}|{:010.1e}|{:.5e}|{:e}|{:.2e}|{:.0e}",
        n, u, n, u, n, n, u, 0u8, big, half
    )
    .unwrap();
    println!("{:?}", chars.0);

    let mut chars = Chars(Vec::new());
    write!(chars, "{}", Closed(-1)).unwrap();
    println!("{:?} {}", chars.0, Closed(3));

    let mut chars = Chars(Vec::new());
    write!(chars, "{}", Shown(-2, "a\"b")).unwrap();
    println!("{:?} {}", chars.0, Shown(3, "c"));
    let mut failing = Failing(Vec::new());
    let r = write!(failing, "{}", Shown(-2, "a\"b"));
    println!("{:?} {:?}", failing.0, r);

    let celled = Celled {
        count: Cell::new(-3),
        names: RefCell::new(vec!["a"]),
    };
    let mut log = Log(Vec::new());
    write!(log, "{:?}|", celled).unwrap();
    let borrowed = celled.names.borrow_mut();
    write!(log, "{:#?}", celled).unwrap();
    drop(borrowed);
    let lone = RefCell::new(Some(2u8));
    write!(log, "|{:?}", lone).unwrap();
    println!("{:?}", log.0);

    let mut chars = Chars(Vec::new());
    write!(
        chars,
        "{:x}|{:#8x}|{:e}|{:.1e}",
        Hexy(255),
        Hexy(10),
        Hexy(1500),
        Hexy(25)
    )
    .unwrap();
    println!("{:?}", chars.0);
}
