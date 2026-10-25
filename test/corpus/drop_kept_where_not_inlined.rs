// A `Drop` whose body is written where it's dropped, but which is also
// named where it isn't called, the drop an adapter over a `Vec` of them
// is given, `$dropEach(loudDrop_drop)`: its function stays, for that (ADR
// 0589, #230).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let b = Loud(3);
    drop(b);
    let v = vec![Loud(4), Loud(5), Loud(6)];
    let first = v.into_iter().map(|l| l.0 * 10).next();
    println!("{first:?} end");
}
