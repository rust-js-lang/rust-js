// A `flatten()` and a `flat_map()` of `Option`s, made as a filter of what
// isn't `None`, given to generic code with their drop (#1240): closed, as
// each `Option` owns the one item it gives, not std's `Flatten` struct's
// fields, which that form doesn't have.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn last_of<I: Iterator>(it: I) -> Option<I::Item> {
    it.last()
}

fn first_of<I: Iterator>(mut it: I) -> Option<I::Item> {
    it.next()
}

fn main() {
    let last = last_of(vec![Some(D(1)), None, Some(D(2)), Some(D(3))].into_iter().flatten());
    println!("last {:?}", last.map(|d| d.0));
    let first = first_of(vec![None, Some(D(4)), Some(D(5))].into_iter().flatten());
    println!("first {:?}", first.map(|d| d.0));
    let mapped = first_of(vec![D(6), D(7), D(8)].into_iter().flat_map(|d| if d.0 > 6 { Some(d) } else { None }));
    println!("mapped {:?}", mapped.map(|d| d.0));
}
