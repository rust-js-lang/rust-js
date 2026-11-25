// Oracle B's own case of enums (ADR 1204): each form JS holds an enum's
// values in, tagged, numbered, of a declared tag, untagged, and generic,
// each made by a function JS calls for its result's shape alone.
pub enum Shape {
    Empty,
    Circle { radius: f64 },
    Square(u32),
}

#[repr(u8)]
pub enum Level {
    Low = 1,
    High = 7,
}

#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum Job {
    Waiting,
    Done { value: u32 },
}

#[cfg_attr(rust_js, rust_js::untagged)]
pub enum Size {
    Named(String),
    Exact(u32),
}

pub enum Held<T> {
    Nothing,
    One(T),
}

pub fn shape(n: u32) -> Shape {
    match n % 3 {
        0 => Shape::Empty,
        1 => Shape::Circle {
            radius: f64::from(n) / 2.0,
        },
        _ => Shape::Square(n),
    }
}

pub fn level(n: u32) -> Level {
    if n > 3 { Level::High } else { Level::Low }
}

pub fn job(n: u32) -> Job {
    if n % 2 == 0 { Job::Waiting } else { Job::Done { value: n } }
}

pub fn size(n: u32) -> Size {
    if n % 2 == 0 { Size::Named(format!("s{n}")) } else { Size::Exact(n) }
}

pub fn held(n: u32) -> Held<u64> {
    if n == 0 { Held::Nothing } else { Held::One(u64::from(n) << 40) }
}

pub fn counted(a: u32, b: u32) -> u64 {
    u64::from(a) * u64::from(b)
}
