//@ compile-fail: `clone_from_slice` of items whose `clone_from` may be the crate's
// `clone_from_slice` of items with a type inside them whose `clone_from` is
// the crate's own: refused, as std's would call it, through std's.

struct Counted(u32);

impl Clone for Counted {
    fn clone(&self) -> Self {
        Counted(self.0)
    }

    fn clone_from(&mut self, source: &Self) {
        println!("clone_from");
        self.0 = source.0;
    }
}

fn main() {
    let source = [vec![Counted(1)]];
    let mut target = [vec![Counted(2)]];
    target.clone_from_slice(&source);
    println!("{}", target[0][0].0);
}
