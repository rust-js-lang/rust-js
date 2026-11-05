// A `&mut` of the crate's iterator given where std asks `IntoIterator`, a
// `flatten()`'s items, a `zip()`'s or a `chain()`'s other side: std's impl
// for every iterator, as rustc's selection says, which takes it as it is,
// and steps it through the `&mut` (#742); beside a `Vec`'s `&mut` and `&`,
// a collection's own.
struct Own(u32);

impl Iterator for Own {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.0 > 0).then(|| {
            self.0 -= 1;
            self.0
        })
    }
}

fn main() {
    let mut a = Own(2);
    let mut b = Own(3);
    let n = vec![&mut a, &mut b].into_iter().flatten().count();
    println!("{n} {} {}", a.0, b.0);
    let mut c = Own(2);
    let zipped: Vec<(u32, u32)> = (0..5).zip(&mut c).collect();
    println!("{zipped:?} {}", c.0);
    let mut d = Own(2);
    let chained: Vec<u32> = (10..12).chain(&mut d).collect();
    println!("{chained:?} {}", d.0);
    let mut v = vec![1, 2];
    for x in &mut v {
        *x += 1;
    }
    let w: Vec<(i32, &i32)> = (0..2).zip(&v).collect();
    println!("{v:?} {w:?}");
}
