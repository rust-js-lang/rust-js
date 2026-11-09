//! Single forms each read across crates as the library reads them (#1032).
use std::panic::Location;

/// A class's constant, its prototype's (ADR 0761).
#[cfg_attr(rust_js, rust_js::class)]
pub struct Counter {
    pub count: u32,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Counter {
    pub const LIMIT: u32 = 10;

    pub fn new() -> Self {
        Counter { count: 0 }
    }
}

/// A trait whose method gives its own associated type, which its
/// dictionary gives generic code as a JS iterator.
pub trait Source {
    type Iter: Iterator<Item = u32>;
    fn numbers(&self) -> Self::Iter;
}

pub struct Upto(pub u32);

impl Source for Upto {
    type Iter = std::ops::Range<u32>;

    fn numbers(&self) -> std::ops::Range<u32> {
        0..self.0
    }
}

pub fn summed<S: Source>(s: &S) -> u32 {
    s.numbers().sum()
}

/// Where it's defined, as a `fn` pointer's call gives it.
#[track_caller]
pub fn here() -> &'static Location<'static> {
    Location::caller()
}

/// A closure given out behind `impl Clone`, which a consumer clones.
pub fn counter() -> impl FnMut() -> i32 + Clone {
    let mut count = 0;
    move || {
        count += 1;
        count
    }
}

/// A `Default` value written again through `&mut self`, its old one dropped.
pub trait Renew {
    fn renew(&mut self);
}

pub struct Slot<T> {
    pub value: T,
}

impl<T: Default> Renew for Slot<T> {
    fn renew(&mut self) {
        self.value = T::default();
    }
}

pub fn renewed<T: Default>(slot: &mut Slot<T>) {
    slot.renew();
}

/// What holds a `T` given on whole to what drops it.
pub fn wipe<T>(slot: Slot<T>) {
    discard(slot);
}

pub fn discard<U>(value: U) {
    let _taken = value;
}

/// A closure that captures nothing, called where it's made.
pub fn through_closure<T>(x: T) -> Option<T> {
    let make = |v: T| Some(v);
    make(x)
}

/// A blanket impl whose `&mut self` method writes it whole.
pub trait Reset {
    fn reset(&mut self);
}

impl<T: Default> Reset for T {
    fn reset(&mut self) {
        *self = T::default();
    }
}

pub fn reset(r: &mut dyn Reset) {
    r.reset();
}
