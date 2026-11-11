// `size_hint()` of a held `filter`, `skip_while`, `take_while`, `map_while`
// or `scan`: `(0, upper)` of its source's, as std's are, a `take_while`'s
// `(0, Some(0))` once it's stopped; a filter of items with a destructor
// too, which drops each it passes, and one an `impl Iterator` gives. Each
// was refused in the build that makes adapters in place (#1169).
#[derive(Debug)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn louds(ns: &[u32]) -> Vec<Loud> {
    ns.iter().map(|&n| Loud(n)).collect()
}

fn evens(v: Vec<Loud>) -> impl Iterator<Item = Loud> {
    v.into_iter().filter(|l| l.0 % 2 == 0)
}

fn dropping() {
    let mut f = louds(&[1, 2, 3, 4]).into_iter().filter(|l| l.0 % 2 == 0);
    println!("{:?}", f.size_hint());
    println!("{:?} {:?}", f.next(), f.size_hint());
    drop(f);
    let mut s = louds(&[5, 6, 7]).into_iter().skip_while(|l| l.0 < 6);
    println!("{:?}", s.size_hint());
    println!("{:?} {:?}", s.next(), s.size_hint());
    drop(s);
    let mut t = louds(&[8, 9, 10]).into_iter().take_while(|l| l.0 < 9);
    println!("{:?}", t.size_hint());
    let first = t.next();
    println!("{first:?} {:?}", t.size_hint());
    let stopped = t.next();
    println!("{stopped:?} {:?}", t.size_hint());
    drop(t);
    let mut e = evens(louds(&[11, 12]));
    println!("{:?} {:?}", e.size_hint(), e.next());
    println!("end dropping");
}

fn plain() {
    let mut s = vec![1u8, 2, 3].into_iter().skip_while(|x| *x < 2);
    println!("{:?} {:?} {:?}", s.size_hint(), s.next(), s.size_hint());
    let mut t = vec![1u8, 2, 3].into_iter().take_while(|x| *x < 2);
    println!("{:?} {:?} {:?}", t.size_hint(), t.next(), t.size_hint());
    println!("{:?} {:?}", t.next(), t.size_hint());
    let mut m = vec![1u8, 2, 3].into_iter().map_while(|x| (x < 3).then_some(x * 2));
    println!("{:?} {:?} {:?}", m.size_hint(), m.next(), m.size_hint());
    let mut c = vec![1u8, 2, 3].into_iter().scan(0u8, |sum, x| {
        *sum += x;
        Some(*sum)
    });
    println!("{:?} {:?} {:?}", c.size_hint(), c.next(), c.size_hint());
    let v = [1, 2, 3];
    println!("{:?}", v.iter().take_while(|x| **x > 1).size_hint());
    println!("{:?}", (1..10).skip_while(|x| *x < 4).size_hint());
}

fn main() {
    dropping();
    plain();
}
