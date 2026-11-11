//@ library-differs
// A `pub` generic function cycles the `Map<I, F>` it makes, of an `I:
// Iterator<Item = i32>` and an `F: FnMut(i32) -> i32`, any types its
// callers may give within those bounds, so it's cloned apart. Another
// generic function's `map(Some)` of items with a destructor, collected
// into an `Option<Vec<_>>`, is no instance of it, as those bounds say: it
// isn't cloned apart, and its items are dropped as Rust drops them. It was
// refused beside the first, "cloning a `Map` .. of items with a destructor"
// (#1318).
pub fn cycled<I, F>(items: I, f: F) -> Vec<i32>
where
    I: Iterator<Item = i32> + Clone,
    F: FnMut(i32) -> i32 + Clone,
{
    items.map(f).cycle().take(5).collect()
}

#[derive(Debug)]
pub struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn tried<I: Iterator<Item = D>>(src: I) -> usize {
    let v: Option<Vec<D>> = src.map(Some).collect();
    v.map_or(0, |v| v.len())
}

fn main() {
    println!("{:?}", cycled(1..3, |x| x * x));
    println!("{}", tried(vec![D(1), D(2)].into_iter()));
    println!("end");
}
