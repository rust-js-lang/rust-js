// A trait whose methods take `&mut self`, declared as the type of its
// dictionaries (#662): its `&mut self` is the box its entry takes, `{
// value: Self }`, and a `&mut` it takes besides as `MutRefForm` says. One
// that gives a `&mut`, or takes one to its `Self` besides, isn't.

pub trait Counter {
    fn bump(&mut self);
    fn add(&self, total: &mut u32);
    fn fill(&self, out: &mut Vec<u32>);
    fn get(&self) -> u32;
    fn reset(&mut self, to: u32) -> u32 {
        let old = self.get();
        for _ in 0..to {
            self.bump();
        }
        old
    }
}

pub struct Clicks {
    pub n: u32,
}

impl Counter for Clicks {
    fn bump(&mut self) {
        self.n += 1;
    }
    fn add(&self, total: &mut u32) {
        *total += self.n;
    }
    fn fill(&self, out: &mut Vec<u32>) {
        out.push(self.n);
    }
    fn get(&self) -> u32 {
        self.n
    }
}

impl Counter for u32 {
    fn bump(&mut self) {
        *self += 1;
    }
    fn add(&self, total: &mut u32) {
        *total += *self;
    }
    fn fill(&self, out: &mut Vec<u32>) {
        out.push(*self);
    }
    fn get(&self) -> u32 {
        *self
    }
}

pub fn bump_twice<C: Counter>(c: &mut C) -> u32 {
    c.bump();
    c.bump();
    c.get()
}

pub fn bump_dyn(c: &mut dyn Counter) -> u32 {
    c.bump();
    c.get()
}

pub fn clicks() -> Clicks {
    Clicks { n: 0 }
}

pub trait Swap {
    fn swap_with(&mut self, other: &mut Self);
}

impl Swap for u32 {
    fn swap_with(&mut self, other: &mut Self) {
        std::mem::swap(self, other);
    }
}

pub trait Slot {
    fn slot(&mut self) -> &mut u32;
}

impl Slot for Clicks {
    fn slot(&mut self) -> &mut u32 {
        &mut self.n
    }
}
