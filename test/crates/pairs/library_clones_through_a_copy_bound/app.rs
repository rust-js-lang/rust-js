use dep::{cloned_of, copied_of, repeated, to_vec_of};

#[derive(Copy, Debug)]
struct Loud(u8);

impl Clone for Loud {
    fn clone(&self) -> Self {
        println!("clone {}", self.0);
        Self(self.0 + 100)
    }
}

#[derive(Clone, Copy, Debug)]
struct Quiet(u8);

// A copy of which is the value itself, its `Clone` still run.
#[derive(Copy)]
struct Unit;

impl Clone for Unit {
    fn clone(&self) -> Self {
        println!("clone unit");
        Unit
    }
}

pub fn main() {
    let louds = [Loud(1), Loud(2)];
    println!("to_vec {:?}", to_vec_of(&louds));
    println!("cloned {:?}", cloned_of(&louds));
    println!("repeat {:?}", repeated(&louds, 3));
    println!("copied {:?}", copied_of(&louds));
    let mut quiets = to_vec_of(&[Quiet(1), Quiet(2)]);
    quiets[0].0 = 9;
    println!("quiet {:?} {:?}", quiets, repeated(&quiets, 2));
    println!("numbers {:?}", cloned_of(&[1u8, 2]));
    println!("units {}", to_vec_of(&[Unit, Unit]).len());
}
