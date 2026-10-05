// `filter` and `map_or` of an `Option` whose value has a destructor: what
// `filter` doesn't keep is dropped once its test says so, and `map_or`'s
// fallback once the function has run, as chrono's do. `ok_or_else` keeps
// its value, as bitflags' parser does.

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

enum Found {
    Nothing,
    One(D),
}

fn make(n: u32) -> Option<D> {
    if n > 0 { Some(D(n)) } else { None }
}

fn even(d: &D) -> bool {
    println!("test {}", d.0);
    d.0 % 2 == 0
}

// In generic code, a `T` whose `Some` is boxed (ADR 0051).
fn keep<T>(value: Option<T>, test: impl Fn(&T) -> bool) -> Option<T> {
    value.filter(test)
}

fn found<T>(value: Option<T>, fallback: T) -> T {
    value.map_or(fallback, |t| t)
}

trait Named: Sized {
    fn from_name(name: &str) -> Option<Self>;
}

impl Named for D {
    fn from_name(name: &str) -> Option<D> {
        name.parse().ok().map(D)
    }
}

fn parse<B: Named>(name: &str) -> Result<B, String> {
    let parsed = B::from_name(name).ok_or_else(|| format!("unknown {name}"))?;
    Ok(parsed)
}

// In a branch, as bitflags' parser has it: dropped there, if a panic comes
// before it's moved.
fn parse_flag<B: Named>(flag: &str) -> Result<B, String> {
    let parsed = if let Some(flag) = flag.strip_prefix("0x") {
        B::from_name(flag).ok_or_else(|| format!("bad hex {flag}"))?
    } else {
        B::from_name(flag).ok_or_else(|| format!("unknown {flag}"))?
    };
    Ok(parsed)
}

// Two in one branch.
fn parse_pair<B: Named>(a: &str, b: &str, both: bool) -> Result<(B, Option<B>), String> {
    let pair = if both {
        (
            B::from_name(a).ok_or_else(|| format!("unknown {a}"))?,
            Some(B::from_name(b).ok_or_else(|| format!("unknown {b}"))?),
        )
    } else {
        (B::from_name(a).ok_or_else(|| format!("unknown {a}"))?, None)
    };
    Ok(pair)
}

fn main() {
    let kept = make(2).filter(even);
    println!("kept {}", kept.is_some());
    let dropped = make(3).filter(even);
    println!("dropped {}", dropped.is_none());
    println!("none {}", make(0).filter(even).is_none());
    let closure = make(5).filter(|d| d.0 > 4);
    println!("closure {}", closure.as_ref().map_or(0, |d| d.0));
    println!("generic {}", keep(make(6), |d| d.0 > 9).is_none());
    println!("generic {}", keep(Some(make(7)), |d| d.is_some()).is_some());

    let one = make(8).map_or(Found::Nothing, Found::One);
    if let Found::One(d) = &one {
        println!("one {}", d.0);
    }
    let nothing = make(0).map_or(Found::One(D(9)), Found::One);
    println!("nothing {}", matches!(nothing, Found::One(_)));
    let read = make(10).map_or(D(11), |d| D(d.0 + 100));
    println!("read {}", read.0);
    println!("found {}", found(make(12), D(13)).0);
    println!("found {}", found(None, D(14)).0);

    let ok = make(15).ok_or_else(|| "none".to_string());
    println!("ok {}", ok.as_ref().map_or(0, |d| d.0));
    let err = make(0).ok_or_else(|| String::from("none"));
    println!("err {}", err.is_err());
    match parse::<D>("16") {
        Ok(d) => println!("parsed {}", d.0),
        Err(e) => println!("{e}"),
    }
    match parse::<D>("x") {
        Ok(d) => println!("parsed {}", d.0),
        Err(e) => println!("{e}"),
    }
    for flag in ["0x17", "18", "0xy", "z"] {
        match parse_flag::<D>(flag) {
            Ok(d) => println!("flag {}", d.0),
            Err(e) => println!("{e}"),
        }
    }
    for (a, b, both) in [("19", "20", true), ("21", "x", true), ("22", "", false)] {
        match parse_pair::<D>(a, b, both) {
            Ok((d, e)) => println!("pair {} {}", d.0, e.map_or(0, |e| e.0)),
            Err(e) => println!("{e}"),
        }
    }
    println!("end");
}
