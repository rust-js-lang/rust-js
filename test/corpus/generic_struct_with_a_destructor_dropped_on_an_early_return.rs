// A generic struct with a `Drop`, `Token<T: Iterator>(T::Item)`, made only in
// generic code, `pairs<I>`, and left in a `Chain`'s `self.1` when its
// by-value `parse` returns early by `?`: dropped there, as Rust drops it.
// A crate whose only values with a destructor are made in generic code
// gives that code their drops (#1650); it gave none, so they leaked.
trait Parser {
    type Input: Iterator;
    fn parse(self, input: Self::Input) -> Result<Self::Input, Self::Input>;
}
struct Token<T: Iterator>(T::Item);
impl<T: Iterator> Drop for Token<T> {
    fn drop(&mut self) {
        println!("drop Token");
    }
}
impl<T: Iterator> Parser for Token<T>
where
    T::Item: PartialEq,
{
    type Input = T;
    fn parse(self, mut input: T) -> Result<T, T> {
        if input.next().is_some_and(|x| x == self.0) {
            Ok(input)
        } else {
            Err(input)
        }
    }
}
struct Chain<L, R>(L, R);
impl<L: Parser, R: Parser<Input = L::Input>> Parser for Chain<L, R> {
    type Input = L::Input;
    fn parse(self, input: L::Input) -> Result<L::Input, L::Input> {
        let rest = self.0.parse(input)?;
        self.1.parse(rest)
    }
}
fn pairs<I: Iterator<Item = i32>>(i: I) -> Result<I, I> {
    Chain(Token(1), Token(2)).parse(i)
}
fn main() {
    println!("{}", pairs([0, 0].into_iter()).is_ok());
}
