// `size_hint()` of a `Fuse` no type of the crate's holds: what it steps',
// and none once it's ended (ADR 0473).
fn hint<I: Iterator>(it: I) -> (usize, Option<usize>) {
    let mut fused = it.fuse();
    fused.next();
    fused.size_hint()
}

// A trait's default given no fact of its `Self`, calling one that fuses
// it: its `Fuse` cleared as it ends, as no caller says (ADR 0543).
trait Ext: Iterator {
    fn hinted(self) -> (usize, Option<usize>)
    where
        Self: Sized,
    {
        hint(self)
    }
}

impl<T: Iterator> Ext for T {}

fn main() {
    let items = [1, 2, 3];
    println!("{:?} {:?}", hint(items.iter()), items.iter().fuse().size_hint());
    println!("{:?}", items.iter().hinted());
}
