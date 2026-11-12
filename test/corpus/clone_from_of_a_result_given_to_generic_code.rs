// `clone_from` of a `Result` through generic code's `Clone` dictionary,
// quickcheck 0.9's `impl<A: Arbitrary, B: Arbitrary> Arbitrary for
// Result<A, B>` (#927): of two `Ok`s the value's own `clone_from`, of two
// `Err`s the error's; otherwise a clone written over it, what it held
// dropped. A `Result` copied before is left as it was.
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

fn assign<T: Clone>(dest: &mut T, source: &T) {
    dest.clone_from(source);
}

fn generic<A: Clone + std::fmt::Debug, B: Clone + std::fmt::Debug>(
    mut dest: Result<A, B>,
    source: Result<A, B>,
) -> Result<A, B> {
    assign(&mut dest, &source);
    dest
}

fn main() {
    let mut w: Result<Counted, Counted> = Ok(Counted(1));
    assign(&mut w, &Ok(Counted(2)));
    println!("{w:?}");
    assign(&mut w, &Err(Counted(3)));
    println!("{w:?}");
    assign(&mut w, &Err(Counted(4)));
    println!("{w:?}");
    println!("{:?}", generic::<Counted, u8>(Ok(Counted(5)), Ok(Counted(6))));
    println!("{:?}", generic::<u8, Counted>(Err(Counted(7)), Err(Counted(8))));
    println!("{:?}", generic::<Counted, u8>(Err(9), Ok(Counted(10))));
    println!("{:?}", generic::<Option<Counted>, ()>(Ok(Some(Counted(11))), Ok(Some(Counted(12)))));
    println!("{:?}", generic::<u32, u32>(Ok(1), Err(2)));
}
