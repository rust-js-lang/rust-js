// A library's trait of classes whose methods give a field of `self`: one
// that only gives it, `&self.n`, has no JS of its own, its field read in
// place wherever it's called (rd304); one that does more first, sets a
// field, calls a method, drops a value, gives another's field or one
// through a reference, or lends it `&mut`, is its prototype's method, as
// its library compiled it, called by its consumer (#1073, #1074).
pub trait Gated {
    fn kept(&self) -> &u32;
    fn mark(&mut self) -> &bool;
    fn later(&self) -> &u32;
    fn guarded(&self) -> &u32;
    fn counter(&mut self) -> &mut u32;
    fn pick<'a>(&'a self, other: &'a Gate) -> &'a u32;
    fn limit(&self) -> &u32;
}

pub struct Limits {
    pub most: u32,
}

static LIMITS: Limits = Limits { most: 40 };

#[cfg_attr(rust_js, rust_js::class)]
pub struct Gate {
    seen: bool,
    n: u32,
    limits: &'static Limits,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Gate {
    pub fn new(n: u32) -> Self {
        Gate {
            seen: false,
            n,
            limits: &LIMITS,
        }
    }

    pub fn note(&self) {
        println!("noted {}", self.n);
    }
}

pub struct Noisy;

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("noisy dropped");
    }
}

impl Gated for Gate {
    fn kept(&self) -> &u32 {
        &self.n
    }

    fn mark(&mut self) -> &bool {
        self.seen = true;
        &self.seen
    }

    fn later(&self) -> &u32 {
        let n = &self.n;
        self.note();
        n
    }

    fn guarded(&self) -> &u32 {
        let _noisy = Noisy;
        &self.n
    }

    fn counter(&mut self) -> &mut u32 {
        &mut self.n
    }

    fn pick<'a>(&'a self, other: &'a Gate) -> &'a u32 {
        &other.n
    }

    fn limit(&self) -> &u32 {
        &self.limits.most
    }
}
