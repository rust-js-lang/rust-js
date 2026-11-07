// `clone_from_slice` of `Vec`s whose items' `clone_from` is the crate's
// own: each `Vec`'s std's `clone_from`, which calls the items' own for
// those it keeps and clones the rest (#919; refused before).

struct Counted(u32);

impl Clone for Counted {
    fn clone(&self) -> Self {
        println!("clone");
        Counted(self.0)
    }

    fn clone_from(&mut self, source: &Self) {
        println!("clone_from");
        self.0 = source.0;
    }
}

fn main() {
    let source = [vec![Counted(1), Counted(3)]];
    let mut target = [vec![Counted(2)]];
    target.clone_from_slice(&source);
    println!("{} {}", target[0][0].0, target[0][1].0);
}
