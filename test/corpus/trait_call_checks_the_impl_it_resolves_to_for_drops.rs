// A call of a trait's method rustc resolves to one impl is checked against
// that impl's own requirement alone (#2276): `Y`'s `via` takes no
// destructor of its `I`, as it folds a box of it, while `Z`'s, the trait's
// default, counts it and drops it, as `Odd`'s `folds` does, overriding a
// default that takes none. So `Z.via(..)`, `Odd.folds(..)` and `ViaTwice`'s
// default copied into `Z`'s, whose `self.via(..)` is `Z`'s, given an
// iterator with a destructor drop it as native Rust does, where generic
// code's call through the trait, which any impl may answer, is refused.
struct D(u32);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}
struct Own(u32);
impl Iterator for Own {
    type Item = D;
    fn next(&mut self) -> Option<D> {
        self.0 = self.0.checked_sub(1)?;
        Some(D(self.0))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.0 as usize, Some(self.0 as usize))
    }
}
impl DoubleEndedIterator for Own {
    fn next_back(&mut self) -> Option<D> {
        self.next()
    }
}
impl ExactSizeIterator for Own {}
impl Drop for Own {
    fn drop(&mut self) {
        println!("drop Own");
    }
}
fn folded<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(it: Box<I>) -> u32 {
    let held = D(91);
    it.fold(0, move |n, x| {
        let _held = &held;
        n + x.0
    })
}
trait Folds {
    fn folds<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(&self, it: Box<I>) -> u32 {
        folded(it)
    }
}
struct Odd;
impl Folds for Odd {
    fn folds<I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(&self, it: Box<I>) -> u32 {
        it.len() as u32 * 2
    }
}
trait Via {
    fn via<U, I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(&self, _u: U, it: Box<I>) -> u32 {
        it.len() as u32
    }
}
struct Y;
impl Via for Y {
    fn via<U, I: DoubleEndedIterator<Item = D> + ExactSizeIterator>(&self, _u: U, it: Box<I>) -> u32 {
        folded(it) + 1
    }
}
struct Z;
impl Via for Z {}
trait ViaTwice: Via {
    fn twice(&self) -> u32 {
        self.via((), Box::new(Own(2))) * 2
    }
}
impl ViaTwice for Z {}
fn main() {
    println!("{}", Z.via(D(7), Box::new(Own(3))));
    println!("{}", Odd.folds(Box::new(Own(2))));
    println!("{}", Z.twice());
    let _ = Y;
}
