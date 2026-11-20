//@ library-differs
// What a `Box<dyn Iterator>` or a `&mut dyn Iterator` of the crate's
// iterator passes on is std's `Box<I>`'s and `&mut I`'s: `next`,
// `size_hint`, `nth`, `next_back`, `nth_back` and `len` as the iterator's
// own, its `count`, `last` and folds the traits' defaults over its `next`
// (#2217). A flatten of boxes, a skip or a chain of one, and one asked
// directly never call the crate's own `count`, `last` or `fold`, which the
// box hides; a sized `Box`, a `&mut` and a `Peekable` pass on what std's do.
struct Own(u32);
impl Iterator for Own {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.0 = self.0.checked_sub(1)?;
        Some(self.0)
    }
    fn count(self) -> usize {
        println!("own count {}", self.0);
        self.0 as usize
    }
    fn last(self) -> Option<u32> {
        println!("own last {}", self.0);
        self.0.checked_sub(1).map(|_| 0)
    }
    fn fold<B, F: FnMut(B, u32) -> B>(mut self, init: B, mut f: F) -> B {
        println!("own fold {}", self.0);
        let mut acc = init;
        while let Some(x) = self.next() {
            acc = f(acc, x);
        }
        acc
    }
    fn nth(&mut self, n: usize) -> Option<u32> {
        println!("own nth {n}");
        for _ in 0..n {
            self.next()?;
        }
        self.next()
    }
}
fn boxes() -> Vec<Box<dyn Iterator<Item = u32>>> {
    vec![Box::new(Own(3))]
}
fn boxed() {
    println!("a {}", boxes().into_iter().flatten().count());
    println!("b {}", boxes().into_iter().flatten().skip(1).count());
    println!("c {}", boxes().into_iter().flatten().chain(Own(2)).count());
    let mut e = boxes().into_iter().flatten();
    e.next();
    println!("e {}", e.count());
    let b: Box<dyn Iterator<Item = u32>> = Box::new(Own(3));
    println!("f {}", b.count());
    let b: Box<dyn Iterator<Item = u32>> = Box::new(Own(3));
    println!("g {:?}", b.last());
    let b: Box<dyn Iterator<Item = u32>> = Box::new(Own(3));
    println!("h {}", b.fold(0, |a, x| a + x));
    let mut b: Box<dyn Iterator<Item = u32>> = Box::new(Own(3));
    println!("i {:?}", b.nth(1));
    println!("j {:?}", boxes().into_iter().flatten().last());
    println!("k {}", boxes().into_iter().flatten().fold(0, |a, x| a + x));
    let sized = vec![Box::new(Own(3))];
    println!("l {}", sized.into_iter().flatten().count());
    let sized = vec![Box::new(Own(3))];
    println!("m {:?}", sized.into_iter().flatten().last());
    let mut own = Own(3);
    let lent: &mut dyn Iterator<Item = u32> = &mut own;
    println!("n {}", lent.count());
    let mut own = Own(3);
    let lent: &mut dyn Iterator<Item = u32> = &mut own;
    println!("p {}", lent.chain(Own(1)).count());
}

fn counted<I: Iterator>(it: I) -> usize {
    it.count()
}
fn lasted<I: Iterator>(it: I) -> Option<I::Item> {
    it.last()
}
fn wrapped() {
    let mut own = Own(3);
    println!("a {}", counted(&mut own));
    let mut own = Own(3);
    println!("b {}", (&mut own).skip(1).count());
    let mut own = Own(3);
    println!("c {:?}", lasted(&mut own));
    let mut own = Own(3);
    println!("d {}", (&mut own).chain(Own(1)).count());
    let (mut x, mut y) = (Own(2), Own(2));
    let lents: Vec<&mut dyn Iterator<Item = u32>> = vec![&mut x, &mut y];
    println!("e {}", lents.into_iter().flatten().count());
    let (mut x, mut y) = (Own(2), Own(2));
    let lents: Vec<&mut dyn Iterator<Item = u32>> = vec![&mut x, &mut y];
    println!("f {:?}", lents.into_iter().flatten().last());
    let mut own = Own(3);
    let lent: &mut dyn Iterator<Item = u32> = &mut own;
    println!("g {}", lent.chain(Own(1)).count());
    println!("h {}", Box::new(Own(3)).count());
    println!("i {:?}", Box::new(Own(3)).last());
    println!("j {}", counted(Box::new(Own(3))));
    println!(
        "k {}",
        vec![Box::new(Own(2)), Box::new(Own(2))].into_iter().flatten().count()
    );
    println!("l {}", Box::new(Own(3)).skip(1).count());
    println!("m {}", Own(3).peekable().count());
    let mut p = Own(3).peekable();
    p.peek();
    println!("n {}", p.count());
    println!("o {:?}", Own(3).peekable().last());
    println!("p {}", Box::new(Own(3).peekable()).count());
    let b: Box<dyn Iterator<Item = u32>> = Box::new(Own(3).peekable());
    println!("q {}", b.count());
    let b: Box<dyn Iterator<Item = u32>> = Box::new(Box::new(Own(3)));
    println!("r {}", b.count());
}

fn main() {
    boxed();
    wrapped();
}
