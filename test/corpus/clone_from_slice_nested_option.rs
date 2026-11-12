// `clone_from_slice` of `Option`s of a type whose `clone_from` is the
// crate's own: each item's `Option::clone_from`, which calls it of two
// `Some`s, as std's does (#919, #927).

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
    let source = [Some(Counted(1))];
    let mut target = [Some(Counted(2))];
    target.clone_from_slice(&source);
    println!("{}", target[0].as_ref().map_or(0, |c| c.0));
}
