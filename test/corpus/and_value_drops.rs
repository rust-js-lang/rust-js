// `and` of an `Option` or a `Result` drops what it gives up: its own
// `Some`'s or `Ok`'s value where it gives back the other, and the other where
// it gives back its own `None` or `Err`, a type parameter's by its given drop
// (ADR 0496).
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn pick<T, U>(a: Option<T>, b: Option<U>) -> Option<U> {
    a.and(b)
}
fn main() {
    let a = Some(Loud(1)).and(Some(Loud(2)));
    println!("a {}", a.map_or(0, |l| l.0));
    let b = None::<Loud>.and(Some(Loud(3)));
    println!("b {}", b.is_none());
    let c: Result<Loud, Loud> = Ok(Loud(4));
    let c = c.and(Ok::<Loud, Loud>(Loud(5)));
    println!("c {}", c.map_or(0, |l| l.0));
    let d: Result<Loud, Loud> = Err(Loud(6));
    let d = d.and(Ok::<Loud, Loud>(Loud(7)));
    println!("d {}", d.is_err());
    let e = pick(Some(Loud(8)), Some(Loud(9)));
    println!("e {}", e.map_or(0, |l| l.0));
    let f = pick(None::<Loud>, Some(Loud(10)));
    println!("f {}", f.is_none());
    println!("{:?}", Some(1).and(Some(2)));
    let g: Result<Loud, u8> = Ok(Loud(11));
    println!("g {:?}", g.and(Ok::<u8, u8>(1)));
}
