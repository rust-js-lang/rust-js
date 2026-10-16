// `repeat(value)` of a value with a destructor (ADR 0422): each item a
// clone, and `value` dropped when the iterator is, `take(n)`'s and
// `skip(n)`'s of it too, in generic code too; and `extend` of what holds
// nothing once it's given all its items.

#[derive(Clone)]
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn fill(v: &mut Vec<Loud>, value: Loud, n: usize) {
    v.extend(std::iter::repeat(value).take(n));
    println!("filled {}", v.len());
}

fn fill_generic<T: Clone>(v: &mut Vec<T>, value: T, n: usize) {
    v.extend(std::iter::repeat(value).take(n));
}

fn main() {
    let mut v = Vec::new();
    fill(&mut v, Loud("a"), 2);
    let mut it = std::iter::repeat(Loud("b")).take(3);
    let first = it.next();
    println!("took {}", first.as_ref().map_or("none", |l| l.0));
    drop(it);
    println!("dropped it");
    let unused = std::iter::repeat(Loud("c")).take(1);
    drop(unused);
    for item in std::iter::repeat(Loud("d")).take(2) {
        println!("got {}", item.0);
    }
    let skipped = std::iter::repeat(Loud("e")).skip(1);
    drop(skipped);
    fill_generic(&mut v, Loud("f"), 1);
    v.extend(vec![Loud("g")]);
    v.extend([Loud("h")]);
    v.extend(Some(Loud("i")));
    v.extend(vec![Loud("j")].into_iter());
    v.extend(std::iter::once(Loud("k")));
    println!("end {}", v.len());
}
