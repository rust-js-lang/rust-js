// Oracle B's own case (ADR 1204): structs JS reads of each field form the
// records say, a plain `Option`'s `None` left out, a present one's written,
// a nullable one's `null`; a transparent struct, its field; and a struct of
// structs, each made by a function JS calls for its result's shape alone.
pub struct Profile {
    pub name: String,
    pub nick: Option<String>,
    #[cfg_attr(rust_js, rust_js::present)]
    pub note: Option<u8>,
    #[cfg_attr(rust_js, rust_js::nullable)]
    pub avatar: Option<u32>,
}

#[repr(transparent)]
pub struct Meters(pub f64);

pub struct Placed {
    pub at: Meters,
    pub by: Profile,
    pub steps: Vec<u64>,
}

pub fn profile(n: u32) -> Profile {
    Profile {
        name: format!("p{n}"),
        nick: (n % 2 == 1).then(|| format!("n{n}")),
        note: (n > 1).then_some(3),
        avatar: (n > 2).then_some(n),
    }
}

// Each `None` written where it's made: a plain one's key left out, a
// present one's kept.
pub fn blank() -> Profile {
    Profile {
        name: String::new(),
        nick: None,
        note: None,
        avatar: None,
    }
}

pub fn meters(x: f64) -> Meters {
    Meters(x * 2.0)
}

pub fn placed(n: u32) -> Placed {
    Placed {
        at: Meters(f64::from(n)),
        by: profile(n),
        steps: (0..u64::from(n % 4)).collect(),
    }
}

pub fn widened(a: u32, b: u32) -> u64 {
    u64::from(a) + u64::from(b)
}
