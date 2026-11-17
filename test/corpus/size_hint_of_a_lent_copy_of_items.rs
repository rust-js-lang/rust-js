// `size_hint()` of an adapter std's struct is, over a `cloned()` or
// `copied()` of an array's, a `Vec`'s or strings' items, lent through a
// function and asked there: the copy, made in place, and its source are
// made knowing their hint where it's asked, the source `$iter`, so the
// struct's hint is std's (#1998). Then stepped, and its hint asked again.
// And what that makes compile beside: a `Vec` grown by a `take(n)` of a
// `cycle()` of a zip of copies or of a map, its capacity std's; a
// `peekable()` of copies' hint, peeked or not; and a lent `scan()` of a
// `map` beside one of a `cloned()` (#1987).
#[derive(Clone, Debug)]
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn pass<I>(it: I) -> I {
    it
}

fn scanned(v: &[D]) -> (usize, Option<usize>) {
    let mut a = v.iter().cloned().scan(0u32, |s, x| {
        *s += 1;
        (*s < 4).then_some(x)
    });
    let b = pass(a.by_ref());
    b.size_hint()
}

fn filtered(v: &[u32]) -> (usize, Option<usize>) {
    let mut a = v.iter().copied().filter(|x| *x > 1);
    let b = pass(a.by_ref());
    let first = b.next();
    println!("first {first:?}");
    b.size_hint()
}

fn names(v: &[String]) -> (usize, Option<usize>) {
    let a = v.iter().cloned().filter(|s| s.len() > 1);
    pass(a).size_hint()
}

fn skipped(v: &Vec<D>) -> (usize, Option<usize>) {
    let mut a = v.iter().cloned().skip(1);
    let b = pass(&mut a);
    let next = b.next();
    println!("next {next:?}");
    b.size_hint()
}

fn cycled(a: &[u8]) -> (usize, usize) {
    let v: Vec<(u8, u8)> = a.iter().copied().zip(a.iter().copied()).cycle().take(4).collect();
    let w: Vec<u8> = a.iter().map(|x| x + 1).cycle().take(5).collect();
    (v.capacity(), w.capacity())
}

fn peeked(v: &[u32]) -> ((usize, Option<usize>), Option<u32>, (usize, Option<usize>)) {
    let hint = v.iter().copied().peekable().size_hint();
    let mut p = v.iter().copied().peekable();
    let first = p.peek().copied();
    (hint, first, p.size_hint())
}

fn mapped(v: &[D]) -> (usize, Option<usize>) {
    let mut a = v.iter().map(|x| x.0).scan(0u32, |s, x| {
        *s += 1;
        (*s < 4).then_some(x)
    });
    let b = pass(a.by_ref());
    b.size_hint()
}

fn main() {
    let ds = [D(1), D(2), D(3), D(4)];
    println!("{:?}", scanned(&ds));
    println!("{:?}", scanned(&[]));
    println!("{:?}", filtered(&[1, 2, 3]));
    let words = vec![String::from("a"), String::from("bc")];
    println!("{:?}", names(&words));
    let v = vec![D(5), D(6), D(7)];
    println!("{:?}", skipped(&v));
    println!("{:?} {:?}", cycled(&[1, 2, 3]), cycled(&[]));
    println!("{:?} {:?}", peeked(&[4, 5]), peeked(&[]));
    println!("{:?}", mapped(&v));
}
