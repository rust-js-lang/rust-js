// An untagged enum whose `Display` writes each variant's one value with
// `{}`, a string, an integer, a `bool` or a `char`, webpack's module id of
// `string | number`: its plain `{}`, and its `to_string()`, is the value in
// JS's template, `${id}`, as JS's text of each is Rust's (#557). Each of
// `write!(f, "{x}")`, `x.fmt(f)` and `f.write_str(x)` writes it so. With
// options, `{:>6}`, it's its `fmt` still: `write!` drops them, where
// `x.fmt(f)` pads. One of an `f64`, whose JS text isn't Rust's, `1e21`, one
// that writes more than the value, one that writes its `Debug`, and one that
// writes it with options, are `fmt` calls still.
use std::collections::HashMap;
use std::fmt;

#[cfg_attr(rust_js, rust_js::untagged)]
enum ModuleId {
    Name(String),
    Number(u32),
}

impl fmt::Display for ModuleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ModuleId::Name(name) => write!(f, "{name}"),
            ModuleId::Number(number) => write!(f, "{number}"),
        }
    }
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Key<'a> {
    Text(&'a str),
    Wide(u64),
    Flag(bool),
}

impl fmt::Display for Key<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Key::Text(text) => f.write_str(text),
            Key::Wide(wide) => wide.fmt(f),
            Key::Flag(flag) => fmt::Display::fmt(flag, f),
        }
    }
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Mark {
    Letter(char),
    Huge(i128),
}

impl fmt::Display for Mark {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mark::Letter(letter) => write!(f, "{}", letter),
            Mark::Huge(huge) => write!(f, "{huge}"),
        }
    }
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Boxed {
    Owned(Box<str>),
    Count(usize),
}

impl fmt::Display for Boxed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Boxed::Owned(owned) => f.write_str(owned),
            Boxed::Count(count) => count.fmt(f),
        }
    }
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Chunk {
    Name(String),
    Index(u8),
}

impl fmt::Display for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Chunk::Name(name) => f.write_str(name),
            Chunk::Index(index) => write!(f, "{index}"),
        }
    }
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Measure {
    Name(String),
    Size(f64),
}

impl fmt::Display for Measure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Measure::Name(name) => write!(f, "{name}"),
            Measure::Size(size) => write!(f, "{size}"),
        }
    }
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Labeled {
    Name(String),
    Number(u32),
}

impl fmt::Display for Labeled {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Labeled::Name(name) => write!(f, "{name}"),
            Labeled::Number(number) => write!(f, "#{number}"),
        }
    }
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Quoted {
    Name(String),
    Number(u32),
}

impl fmt::Display for Quoted {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Quoted::Name(name) => write!(f, "{name:?}"),
            Quoted::Number(number) => write!(f, "{number}"),
        }
    }
}

#[cfg_attr(rust_js, rust_js::untagged)]
enum Padded {
    Name(String),
    Number(u32),
}

impl fmt::Display for Padded {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Padded::Name(name) => write!(f, "{name}"),
            Padded::Number(number) => write!(f, "{number:>3}"),
        }
    }
}

fn missing(id: &ModuleId) -> String {
    format!("Could not find the module \"{id}\" in the manifest.")
}

fn exports(manifest: &HashMap<String, &'static str>, id: &ModuleId) -> &'static str {
    manifest.get(&id.to_string()).copied().unwrap_or("none")
}

pub fn main() {
    println!("{}", missing(&ModuleId::Name("./app.js".to_string())));
    println!("{}", missing(&ModuleId::Number(7)));
    println!("{}", ModuleId::Number(4_000_000_000));
    let manifest = HashMap::from([("12".to_string(), "button"), ("./app.js".to_string(), "app")]);
    println!("{}", exports(&manifest, &ModuleId::Number(12)));
    println!("{}", exports(&manifest, &ModuleId::Name("./app.js".to_string())));
    println!("{}", exports(&manifest, &ModuleId::Number(13)));
    // With options, its `fmt`: `write!` drops them, `x.fmt(f)` pads.
    println!("[{:>6}] [{:<4}]", ModuleId::Number(7), ModuleId::Name("ab".to_string()));
    println!(
        "[{:>6}] [{:>6}] [{:>6}]",
        Key::Wide(7),
        Key::Text("ab"),
        Key::Flag(true)
    );
    println!("[{:+}] [{:+}]", Key::Wide(7), ModuleId::Number(7));
    for key in [Key::Text("chunk"), Key::Wide(u64::MAX), Key::Flag(false)] {
        println!("{key} {}", key.to_string().len());
    }
    for mark in [Mark::Letter('é'), Mark::Huge(-(1 << 100))] {
        println!("<{mark}>");
    }
    for boxed in [Boxed::Owned("owned".into()), Boxed::Count(65_536)] {
        println!("<{boxed}>");
    }
    for chunk in [Chunk::Name("main".to_string()), Chunk::Index(255)] {
        println!("chunk {chunk}");
    }
    println!("{} {}", Measure::Name("w".to_string()), Measure::Size(1e21));
    println!("{} {}", Labeled::Name("l".to_string()), Labeled::Number(3));
    println!("{} {}", Quoted::Name("q".to_string()), Quoted::Number(5));
    println!("{} {}", Padded::Name("p".to_string()), Padded::Number(4));
}
