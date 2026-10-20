// `splice` of values with a destructor owns what it takes out, as `drain`
// does, dropped as it's dropped or given each; and `inspect` of an
// `Option` or a `Result` gives back what it's given (ADR 0501).
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
fn main() {
    let mut v = vec![Loud(1), Loud(2), Loud(3)];
    v.splice(0..2, vec![Loud(4)]);
    println!("{:?}", v.iter().map(|l| l.0).collect::<Vec<_>>());
    let removed: Vec<Loud> = v.splice(1.., vec![Loud(5), Loud(6)]).collect();
    println!("{} {:?}", removed.len(), v.iter().map(|l| l.0).collect::<Vec<_>>());
    drop(removed);
    let mut it = v.splice(..1, Vec::new());
    println!("{:?}", it.next().map(|l| l.0));
    drop(it);
    println!("{:?}", v.iter().map(|l| l.0).collect::<Vec<_>>());
    let o = Some(Loud(7)).inspect(|l| println!("saw {}", l.0));
    println!("{:?}", o.map(|l| l.0));
    let r: Result<Loud, u8> = Ok(Loud(8));
    let r = r
        .inspect(|l| println!("ok {}", l.0))
        .inspect_err(|e| println!("err {e}"));
    println!("{}", r.is_ok());
}
