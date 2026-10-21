// A `copied()` or a `cloned()` of an iterator with a destructor, consumed
// to its end by `collect()` or `sum()`, drops it as the method ends, as
// Rust's does; one stopped early, `take(1)`'s, as it stops (ADR 0547).
struct Loud<'a>(&'static str, std::slice::Iter<'a, u32>);

impl<'a> Iterator for Loud<'a> {
    type Item = &'a u32;

    fn next(&mut self) -> Option<&'a u32> {
        self.1.next()
    }
}

impl Drop for Loud<'_> {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let v = [1, 2, 3];
    let all: Vec<u32> = Loud("copied", v.iter()).copied().collect();
    println!("{all:?}");
    let first: Vec<u32> = Loud("cloned", v.iter()).cloned().take(1).collect();
    println!("{first:?}");
    let total: u32 = Loud("summed", v.iter()).copied().sum();
    println!("{total}");
}
