// Generic code that lends what it's given, `it.by_ref()` or `&mut it`,
// steps it where it is, and its first `None` drops nothing: the given
// iterator with a `Drop` is dropped where the function's `it` goes, as
// Rust drops it, never as it first ends, nor stepped after (#1508).
struct Flaky {
    n: u32,
}
impl Iterator for Flaky {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.n += 1;
        if self.n == 3 || self.n > 5 { None } else { Some(self.n) }
    }
}
impl Drop for Flaky {
    fn drop(&mut self) {
        println!("drop Flaky {}", self.n);
    }
}
fn f() -> Flaky {
    Flaky { n: 0 }
}
fn by_next<I: Iterator<Item = u32>>(mut it: I) -> u32 {
    let mut total = 0;
    while let Some(x) = it.next() {
        total += x;
    }
    println!("first part {total}");
    while let Some(x) = it.next() {
        total += x;
    }
    println!("second part {total}");
    total
}
fn by_ref_sum<I: Iterator<Item = u32>>(mut it: I) -> u32 {
    let a: u32 = it.by_ref().sum();
    println!("a {a}");
    let b: u32 = it.by_ref().sum();
    println!("b {b}");
    a + b
}
fn by_for<I: Iterator<Item = u32>>(mut it: I) -> u32 {
    let mut total = 0;
    for x in it.by_ref() {
        total += x;
    }
    println!("for a {total}");
    for x in &mut it {
        total += x;
    }
    println!("for b {total}");
    total
}
fn twice<I: Iterator<Item = u32>>(mut it: I) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let a: Vec<u32> = it.by_ref().collect();
    let b: Vec<u32> = it.by_ref().collect();
    let c: Vec<u32> = it.collect();
    (a, b, c)
}
fn main() {
    println!("{}", by_next(f().map(|x| x + 1)));
    println!("{}", by_next(f()));
    println!("{}", by_next(f().skip(1)));
    println!("{}", by_ref_sum(f().map(|x| x + 1)));
    println!("{}", by_ref_sum(f().skip(1)));
    println!("{}", by_for(f().map(|x| x + 1)));
    println!("{}", by_for(f().filter(|x| x % 2 == 0)));
    let skipped = twice(f().map(|x| x + 1).skip(3));
    println!("map skip3 {skipped:?}");
    let skipped = twice(f().skip(3));
    println!("skip3 {skipped:?}");
    let mut it = f().map(|x| x + 1).skip(3);
    let a: Vec<u32> = it.by_ref().collect();
    println!("{a:?}");
    let b: Vec<u32> = it.by_ref().collect();
    println!("{b:?}");
}
