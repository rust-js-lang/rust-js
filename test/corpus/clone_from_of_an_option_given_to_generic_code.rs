// `clone_from` of an `Option` through generic code's `Clone` dictionary,
// quickcheck 0.9's `impl<A: Arbitrary> Arbitrary for Option<A>` (#927): of
// two `Some`s, the value's own `clone_from`; otherwise a clone written over
// it. Of the crate's `Counted`, of a type parameter's value, boxed as it may
// look like `None`, an `Option<Option<Counted>>`, and of a number; and
// of a value cloned whole, what it held dropped, not what's written.
#[derive(Debug)]
struct Counted(u32);

impl Clone for Counted {
    fn clone(&self) -> Counted {
        println!("clone {}", self.0);
        Counted(self.0)
    }

    fn clone_from(&mut self, source: &Counted) {
        println!("clone_from {} <- {}", self.0, source.0);
        self.0 = source.0;
    }
}

impl Drop for Counted {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Clone, Debug)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop loud {}", self.0);
    }
}

fn assign<T: Clone>(dest: &mut T, source: &T) {
    dest.clone_from(source);
}

fn generic<A: Clone + std::fmt::Debug>(mut dest: Option<A>, source: Option<A>) -> Option<A> {
    assign(&mut dest, &source);
    dest
}

fn main() {
    let mut w = Some(Counted(1));
    assign(&mut w, &Some(Counted(2)));
    println!("{w:?}");
    assign(&mut w, &None);
    println!("{w:?}");
    assign(&mut w, &Some(Counted(3)));
    println!("{w:?}");
    println!("{:?}", generic(Some(Counted(4)), Some(Counted(5))));
    println!("{:?}", generic(None, Some(Counted(6))));
    println!("{:?}", generic(Some(Some(Counted(7))), Some(Some(Counted(8)))));
    println!("{:?}", generic(Some(None), Some(Some(Counted(9)))));
    println!("{:?}", generic(Some(Some(Counted(10))), Some(None)));
    println!("{:?}", generic(Some(1u32), Some(2)));
    println!("{:?}", generic(Some(Loud(1)), Some(Loud(2))));
    println!("{:?}", generic::<()>(Some(()), Some(())));
}
