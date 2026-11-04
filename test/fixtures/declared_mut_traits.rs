// A trait whose methods take `&mut self`, declared as the type of its
// dictionaries (#662): its `&mut self` is the box its entry takes, `{
// value: Self }`, and a `&mut` it takes besides, a `&mut Self` too, or
// gives, as `MutRefForm` says (#679), and one within an `Option` it takes,
// `Option<&mut Self>`, which each entry adapts (#686). One with a `&mut
// Self` elsewhere within what it takes, `Vec<&mut Self>`, isn't (#682).

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

impl Swap for Clicks {
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

impl Slot for u32 {
    fn slot(&mut self) -> &mut u32 {
        self
    }
}

pub trait Keep: Sized {
    fn keep(&mut self, into: &mut Option<Self>) -> bool;
}

impl Keep for Clicks {
    fn keep(&mut self, into: &mut Option<Self>) -> bool {
        let had = into.is_some();
        *into = Some(Clicks { n: self.n + 1 });
        had
    }
}

impl Keep for u32 {
    fn keep(&mut self, into: &mut Option<Self>) -> bool {
        let had = into.is_some();
        *into = Some(*self + 1);
        had
    }
}

pub trait Pick {
    fn pick(&mut self, other: Option<&mut Self>) -> bool;
}

impl Pick for u32 {
    fn pick(&mut self, other: Option<&mut Self>) -> bool {
        other.is_some()
    }
}

pub trait Gather {
    fn gather(&mut self, others: Vec<&mut Self>) -> usize;
}

impl Gather for u32 {
    fn gather(&mut self, others: Vec<&mut Self>) -> usize {
        others.len()
    }
}
