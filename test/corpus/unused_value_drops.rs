// A value std is given and leaves unused is dropped where it's left, on
// the variant of its subject it leaves it on, as a function it leaves unrun
// is: `unwrap_or`'s default of a `Some`, `ok_or`'s and `or`'s, a `Result`'s
// `unwrap_or` and `or` of an `Ok`, and `then_some`'s of `false`; a
// `Result`'s `Err` they give up dropped too, and a type parameter's by its
// given drop (ADR 0490).
struct Loud(u8);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn pick<T>(o: Option<T>, d: T) -> T {
    o.unwrap_or(d)
}

fn maybe<T>(keep: bool, v: T) -> Option<T> {
    keep.then_some(v)
}

struct Gen(u8);

impl Iterator for Gen {
    type Item = Loud;

    fn next(&mut self) -> Option<Loud> {
        self.0 += 1;
        if self.0 < 3 { Some(Loud(self.0 + 30)) } else { None }
    }
}

fn first_or<I: Iterator>(mut it: I, fallback: I::Item) -> I::Item {
    it.next().unwrap_or(fallback)
}

fn main() {
    let a = Some(Loud(1)).unwrap_or(Loud(2));
    println!("a {}", a.0);
    let b = None.unwrap_or(Loud(3));
    println!("b {}", b.0);
    let c = true.then_some(Loud(4));
    println!("c {}", c.map_or(0, |l| l.0));
    let c2 = false.then_some(Loud(5));
    println!("c2 {}", c2.is_none());
    let d: Result<u8, Loud> = Some(6).ok_or(Loud(7));
    println!("d {:?}", d.ok());
    let e = Some(Loud(8)).map_or(Loud(9), |l| l);
    println!("e {}", e.0);
    let f = Some(Loud(10)).or(Some(Loud(11)));
    println!("f {}", f.map_or(0, |l| l.0));
    let g: Result<Loud, u8> = Err(1);
    let g = g.unwrap_or(Loud(12));
    println!("g {}", g.0);
    let h: Result<Loud, u8> = Ok(Loud(13));
    let h = h.map_or(Loud(14), |l| l);
    println!("h {}", h.0);
    let i: Result<Loud, u8> = Ok(Loud(15));
    let i = i.or(Ok::<Loud, u8>(Loud(16)));
    println!("i {}", i.map_or(0, |l| l.0));

    let a = Err::<u8, Loud>(Loud(20)).unwrap_or(3);
    println!("a {a}");
    let b = Err::<u8, Loud>(Loud(21)).or(Ok::<u8, Loud>(4));
    println!("b {:?}", b.ok());
    let c = Ok::<u8, Loud>(5).or(Err::<u8, Loud>(Loud(22)));
    println!("c {:?}", c.ok());
    let p = pick(Some(Loud(23)), Loud(24));
    println!("p {}", p.0);
    let q = pick(None, Loud(25));
    println!("q {}", q.0);
    println!("n {}", pick(None, 7));
    let m = maybe(false, Loud(26));
    println!("m {}", m.is_none());
    let k = maybe(true, Loud(27));
    println!("k {}", k.map_or(0, |l| l.0));
    let first = first_or(Gen(0), Loud(40));
    println!("first {}", first.0);
    let empty = first_or(Gen(5), Loud(41));
    println!("empty {}", empty.0);
}
