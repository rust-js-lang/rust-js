// An untagged enum (ADR 0214): a value is its payload, and a variant is
// told by its runtime kind, a string, a number, a BigInt, a `bool`, an
// array, an object, as TS's `string | number | ..` is.

#[derive(Clone, Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Debug, PartialEq)]
enum Value {
    Text(String),
    Count(i32),
    Big(i64),
    Flag(bool),
    List(Vec<i32>),
    At(Point),
}

impl From<String> for Value {
    fn from(text: String) -> Self {
        Value::Text(text)
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Count(n)
    }
}

#[derive(Clone, Copy, Debug)]
struct Pos {
    x: i32,
}

// A `Copy` one: a copy of its struct is the struct's own.
#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Clone, Copy, Debug)]
enum Spot {
    At(Pos),
    Code(i32),
}

struct Noisy(&'static str);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

// One whose payload has a destructor, run as the variant's.
#[cfg_attr(rust_js, rust_js::untagged)]
enum Held {
    Loud(Noisy),
    Quiet(u32),
    Both(Duo),
}

struct Duo {
    first: Noisy,
    second: Noisy,
}

const START: Value = Value::Count(3);

// What's left of it, once a part's moved out, is dropped as the variant's.
struct Pair {
    held: Held,
    other: Noisy,
}

// A name compared as its own `==` says, whatever its case.
struct Name(String);

impl PartialEq for Name {
    fn eq(&self, other: &Name) -> bool {
        self.0.to_lowercase() == other.0.to_lowercase()
    }
}

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(PartialEq)]
enum Key {
    Named(Name),
    Id(u32),
}

fn describe(value: &Value) -> String {
    match value {
        Value::Count(n) if *n < 0 => format!("negative {n}"),
        Value::Count(n) => format!("count {n}"),
        Value::Text(t) => format!("text {t} of {}", t.len()),
        Value::Big(b) => format!("big {}", b * 2),
        Value::Flag(true) => "yes".to_string(),
        Value::Flag(false) => "no".to_string(),
        Value::List(items) => format!("list of {}", items.len()),
        Value::At(Point { x, y }) => format!("at {x},{y}"),
    }
}

fn main() {
    let mut values = vec![
        Value::At(Point { x: 1, y: 2 }),
        Value::Text("hi".to_string()),
        "there".to_string().into(),
        7.into(),
        Value::Count(-3),
        Value::Big(1 << 40),
        Value::Flag(true),
        Value::Flag(false),
        Value::List(vec![1, 2]),
    ];
    for value in &values {
        println!("{} | {value:?}", describe(value));
    }

    // The payload is the value, so writing it writes the enum's.
    if let Value::List(items) = &mut values[8] {
        items.push(3);
    }
    let copy = values[8].clone();
    values[8] = Value::List(vec![]);
    println!("{copy:?} {:?} {}", values[8], copy == Value::List(vec![1, 2, 3]));

    let counts: Vec<Value> = [4, 5].into_iter().map(Value::Count).collect();
    let texts = values.iter().filter(|v| matches!(v, Value::Text(_))).count();
    let first: Option<&Value> = values.iter().find(|v| matches!(v, Value::Big(_)));
    println!("{counts:?} {texts} {first:?} {}", values[0] == Value::At(Point { x: 1, y: 2 }));

    // A clone is the payload's own, which a change to the first leaves.
    let mut list = Value::List(vec![1]);
    let kept = list.clone();
    if let Value::List(items) = &mut list {
        items.push(2);
    }
    let mut spot = Spot::At(Pos { x: 1 });
    let copied = spot;
    if let Spot::At(p) = &mut spot {
        p.x = 9;
    }
    let code = match Spot::Code(2) {
        Spot::Code(c) => c,
        Spot::At(p) => p.x,
    };
    println!("{list:?} {kept:?} {spot:?} {copied:?} {code} {START:?}");

    let same = Key::Named(Name("Ada".to_string())) == Key::Named(Name("ADA".to_string()));
    let other = Key::Id(1) == Key::Named(Name("1".to_string()));
    println!("{same} {other} {}", Key::Id(2) == Key::Id(2));

    let lone = Held::Loud(Noisy("b"));
    if let Held::Quiet(n) = lone {
        println!("quiet {n}");
    }
    let pair = Pair { held: Held::Loud(Noisy("p")), other: Noisy("q") };
    let other = pair.other;
    println!("moved {}", other.0);
    if let Held::Quiet(n) = &pair.held {
        println!("quiet {n}");
    }
    // Its payload's part moved out: the rest is dropped as the variant's.
    let both = Held::Both(Duo { first: Noisy("f"), second: Noisy("s") });
    if let Held::Both(Duo { first, .. }) = both {
        println!("took {}", first.0);
    }
    let held = [Held::Quiet(1), Held::Loud(Noisy("a"))];
    for h in &held {
        match h {
            Held::Quiet(n) => println!("quiet {n}"),
            Held::Loud(noisy) => println!("loud {}", noisy.0),
            Held::Both(duo) => println!("both {} {}", duo.first.0, duo.second.0),
        }
    }
}
