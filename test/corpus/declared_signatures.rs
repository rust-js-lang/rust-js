// What each exported function takes and gives, as its declaration says it
// (ADR 1091): a `&mut` of a number, a `String`, an `Option` and a `Vec`, a
// struct and a `&mut` of one, a slice, a trailing `Option`, a tuple, an
// `impl Fn`, a type parameter bound by `Fn` (#661) and one bound by `Clone`;
// and of a type parameter bound by a trait of the crate's, given an impl's
// dictionary, whose method gives a `&mut`, or takes a `&mut Self` besides
// its receiver (#679).
// `test/declarations-conformance.test.ts` calls each from TypeScript.

pub struct Point {
    pub x: f64,
    pub y: f64,
}

pub fn bump(n: &mut u32, by: u32) -> u32 {
    *n += by;
    *n
}

pub fn shout(text: &mut String) {
    text.push('!');
}

pub fn fill(slot: &mut Option<u32>, with: u32) -> bool {
    let was = slot.is_some();
    *slot = Some(with);
    was
}

pub fn grow(items: &mut Vec<u32>, n: u32) -> usize {
    items.push(n);
    items.len()
}

pub fn moved(point: &mut Point, by: f64) {
    point.x += by;
    point.y -= by;
}

pub fn length(point: &Point) -> f64 {
    (point.x * point.x + point.y * point.y).sqrt()
}

pub fn origin(x: f64) -> Point {
    Point { x, y: 0.0 }
}

pub fn total(items: &[u32]) -> u32 {
    items.iter().sum()
}

pub fn greet(name: &str, greeting: Option<String>) -> String {
    format!("{}, {name}", greeting.unwrap_or_else(|| "Hello".to_string()))
}

pub fn split(n: u32) -> (u32, bool) {
    (n / 2, n % 2 == 1)
}

pub fn apply(f: impl Fn(u32) -> u32, n: u32) -> u32 {
    f(n) + 1
}

pub fn twice<F: Fn(u32) -> u32>(f: F, n: u32) -> u32 {
    f(f(n))
}

pub fn each_len<F>(items: &[String], mut f: F)
where
    F: FnMut(usize),
{
    for item in items {
        f(item.len());
    }
}

pub fn first_or<T: Clone>(items: &[T], or: T) -> T {
    items.first().cloned().unwrap_or(or)
}

pub struct Tally {
    pub count: u32,
}

pub trait Slot {
    fn slot(&mut self) -> &mut u32;
}

impl Slot for Tally {
    fn slot(&mut self) -> &mut u32 {
        &mut self.count
    }
}

impl Slot for u32 {
    fn slot(&mut self) -> &mut u32 {
        self
    }
}

pub trait Swap {
    fn swap_with(&mut self, other: &mut Self);
}

impl Swap for Tally {
    fn swap_with(&mut self, other: &mut Self) {
        std::mem::swap(self, other);
    }
}

impl Swap for u32 {
    fn swap_with(&mut self, other: &mut Self) {
        std::mem::swap(self, other);
    }
}

pub fn add_to<S: Slot>(s: &mut S, by: u32) -> u32 {
    *s.slot() += by;
    *s.slot()
}

pub fn slot_of<S: Slot>(s: &mut S) -> &mut u32 {
    s.slot()
}

pub fn swapped<C: Swap>(a: &mut C, b: &mut C) {
    a.swap_with(b);
}

fn main() {
    let mut n = 1;
    println!("{}", bump(&mut n, 2));
    let mut text = String::from("hi");
    shout(&mut text);
    let mut slot = None;
    println!("{text} {} {:?}", fill(&mut slot, 4), slot);
    let mut items = vec![1];
    println!("{}", grow(&mut items, 5));
    let mut point = origin(3.0);
    moved(&mut point, 1.0);
    println!("{} {} {}", point.x, point.y, length(&point));
    println!("{} {} {:?}", total(&items), greet("Ann", None), split(7));
    println!("{} {}", apply(|n| n * 3, 2), twice(|n| n + 4, 1));
    let mut lens = Vec::new();
    each_len(&["ab".to_string(), "c".to_string()], |len| lens.push(len));
    println!("{lens:?} {} {}", first_or(&[2, 9], 4), first_or::<u8>(&[], 7));
    let mut tally = Tally { count: 1 };
    let mut other = Tally { count: 8 };
    *slot_of(&mut tally) += 2;
    println!("{} {}", add_to(&mut tally, 3), add_to(&mut n, 4));
    swapped(&mut tally, &mut other);
    swapped(&mut n, &mut 9);
    println!("{} {} {n}", tally.count, other.count);
}
