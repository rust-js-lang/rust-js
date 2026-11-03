// What a take of a reversed `Vec`, or of the crate's own iterator, drops,
// in std's order (#493): an `enumerate`'s `count` is its source's, which a
// take's `advance_by` gives a reversed `Vec`'s slice, dropped front to back;
// a `map` by what gives each as it is is std's `Map`, whose `nth` steps
// each from the back; and a `last` folds what it's given, each before the
// last dropped as the next comes, before what the source holds at its end.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds() -> Vec<D> {
    vec![D(1), D(2), D(3), D(4)]
}

struct Own(Vec<D>);

impl Iterator for Own {
    type Item = D;

    fn next(&mut self) -> Option<D> {
        if self.0.is_empty() {
            None
        } else {
            Some(self.0.remove(0))
        }
    }

    fn nth(&mut self, n: usize) -> Option<D> {
        println!("nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }
}

fn main() {
    let counted = ds().into_iter().rev().fuse().take(2).enumerate().skip(0).count();
    println!("count {counted}");
    let nth = ds().into_iter().rev().fuse().take(3).map(|d| d).nth(2);
    println!("nth {:?}", nth.map(|d| d.0));
    let last = Own(ds()).take(2).enumerate().skip(0).last();
    println!("last {:?}", last.map(|(_, d)| d.0));
    let last = Own(ds()).take(3).skip(0).last();
    println!("last of skip {:?}", last.map(|d| d.0));
}
