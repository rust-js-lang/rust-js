// `xor` and `zip` of `Option`s drop what they give up: `xor` both where both
// are `Some`, `zip` the one that is where the other isn't (ADR 0498).
#[derive(Debug)]
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let a = Some(Loud(1)).xor(Some(Loud(2)));
    println!("a {}", a.is_none());
    let b = Some(Loud(3)).xor(None);
    println!("b {:?}", b.as_ref().map(|l| l.0));
    let c = None.xor(Some(Loud(4)));
    println!("c {:?}", c.as_ref().map(|l| l.0));
    let d = Some(Loud(5)).zip(None::<Loud>);
    println!("d {}", d.is_none());
    let e = None::<Loud>.zip(Some(Loud(6)));
    println!("e {}", e.is_none());
    let f = Some(Loud(7)).zip(Some(8));
    println!("f {:?}", f.as_ref().map(|p| (p.0.0, p.1)));
    println!("{:?}", Some(1).xor(Some(2)));
}
