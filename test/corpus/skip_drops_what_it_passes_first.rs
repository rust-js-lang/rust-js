// A `skip(n)` of items with a destructor drops each it skips as its first
// step passes it, of a source with no `nth` of its own: generic code's,
// given a `Vec`'s items, a `rev()`'s, and a slice's items mapped, a JS
// iterator's own `map`, whose skipped items it drops one by one.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn second<I: Iterator<Item = D>>(items: I) -> Option<u32> {
    items.skip(2).next().map(|d| d.0)
}

fn main() {
    let ds: Vec<D> = (1..6).map(D).collect();
    println!("generic {:?}", second(ds.into_iter()));
    let ds: Vec<D> = (1..6).map(D).collect();
    let mut it = ds.into_iter().rev().skip(1);
    println!("rev {:?}", it.next().map(|d| d.0));
    drop(it);
    let nums = [1u32, 2, 3, 4];
    let mut made = nums.iter().map(|&n| D(n + 10)).skip(2);
    println!("made {:?}", made.next().map(|d| d.0));
    drop(made);
    println!("ended");
}
