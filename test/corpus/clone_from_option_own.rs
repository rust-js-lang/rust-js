// `clone_from` of an `Option` of a type whose `clone_from` is the crate's
// own: of two `Some`s that `clone_from`, as std's `Option::clone_from`
// calls it, never `*dest = source.clone()` (#922, #927).

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
    let source = Some(Counted(1));
    let mut target = Some(Counted(2));
    target.clone_from(&source);
    println!("{}", target.map_or(0, |c| c.0));
}
