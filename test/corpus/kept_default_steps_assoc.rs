//@ library-differs
// A trait's default kept by a blanket impl steps `Self::Iter`, which the
// impl says is its `I`: a range, the crate's iterator or a `Vec`'s, each
// stepped by the dictionary it's given, as `I` is stepped (ADR 0977).

trait Digits: Sized {
    type Iter: Iterator<Item = u8>;
    fn digit_iter(self) -> Self::Iter;
    fn digit_sum(self) -> u32 {
        self.digit_iter().map(|d: u8| d as u32).fold(0, |s, d| s + d)
    }
}
impl<I> Digits for I
where
    I: Iterator<Item = u8>,
{
    type Iter = I;
    fn digit_iter(self) -> I {
        self
    }
}
struct Count(u8);
impl Iterator for Count {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
}
fn main() {
    println!("{}", (1u8..4).digit_sum());
    println!("{}", Count(4).digit_sum());
    println!("{}", vec![1u8, 2].into_iter().digit_sum());
}
