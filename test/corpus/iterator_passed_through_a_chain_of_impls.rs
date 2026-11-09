//@ library-differs
// An `I: Iterator` generic code only passes through a chain of the crate's
// impls, whose associated type bound by `Iterator` it is, and gives back:
// made of each impl's dictionary down to the one given `I` as it is, which
// asks its `Iterator`'s, so the caller gives it (#1062), through an impl's
// bound of any lifetime, `for<'a> Fn(&'a T::Item)`, too. Then stepped by
// what's given it back.
trait Parser {
    type Input: Iterator;
    type Output;
    fn parse(self, input: Self::Input) -> Result<(Self::Output, Self::Input), Self::Input>;
    fn chain<P>(self, p: P) -> Chain<Self, P>
    where
        Self: Sized,
    {
        Chain(self, p)
    }
}

struct Token<T: Iterator>(T::Item);

impl<T: Iterator> Parser for Token<T>
where
    T::Item: PartialEq,
{
    type Input = T;
    type Output = T::Item;
    fn parse(self, mut input: T) -> Result<(T::Item, T), T> {
        match input.next() {
            Some(item) if item == self.0 => Ok((item, input)),
            _ => Err(input),
        }
    }
}

/// Parses nothing, its input given back as it is.
struct Nothing<T>(std::marker::PhantomData<T>);

impl<T: Iterator> Parser for Nothing<T> {
    type Input = T;
    type Output = ();
    fn parse(self, input: T) -> Result<((), T), T> {
        Ok(((), input))
    }
}

fn nothing<T>() -> Nothing<T> {
    Nothing(std::marker::PhantomData)
}

/// Parses an item its test, of any lifetime, keeps.
struct Satisfy<T, F>(F, std::marker::PhantomData<T>);

impl<T: Iterator, F> Parser for Satisfy<T, F>
where
    F: for<'a> Fn(&'a T::Item) -> bool,
{
    type Input = T;
    type Output = T::Item;
    fn parse(self, mut input: T) -> Result<(T::Item, T), T> {
        match input.next() {
            Some(item) if (self.0)(&item) => Ok((item, input)),
            _ => Err(input),
        }
    }
}

struct Chain<L, R>(L, R);

impl<L: Parser, R: Parser<Input = L::Input>> Parser for Chain<L, R> {
    type Input = L::Input;
    type Output = (L::Output, R::Output);
    fn parse(self, input: L::Input) -> Result<((L::Output, R::Output), L::Input), L::Input> {
        let (a, rest) = self.0.parse(input)?;
        let (b, rest) = self.1.parse(rest)?;
        Ok(((a, b), rest))
    }
}

fn pairs<I>(i: I) -> Result<(((i32, i32), (i32, i32)), I), I>
where
    I: Iterator<Item = i32>,
{
    Chain(Token(1), Token(2)).chain(Chain(Token(3), Token(4))).parse(i)
}

fn untouched<I>(i: I) -> Result<((), I), I>
where
    I: Iterator<Item = i32>,
{
    Chain(nothing(), nothing())
        .chain(Chain(nothing(), nothing()))
        .parse(i)
        .map(|(_, i)| ((), i))
}

fn even_then_odd<I>(i: I) -> Result<((i32, i32), I), I>
where
    I: Iterator<Item = i32>,
{
    Chain(
        Satisfy(|x: &i32| x % 2 == 0, std::marker::PhantomData),
        Satisfy(|x: &i32| x % 2 == 1, std::marker::PhantomData),
    )
    .parse(i)
}

fn main() {
    match even_then_odd(vec![4, 7, 8].into_iter()) {
        Ok((pair, rest)) => println!("{pair:?} then {:?}", rest.collect::<Vec<_>>()),
        Err(rest) => println!("no, {:?}", rest.collect::<Vec<_>>()),
    }
    if let Ok(((), rest)) = untouched(Some(7).into_iter()) {
        println!("untouched {:?}", rest.collect::<Vec<_>>());
    }
    match pairs(vec![1, 2, 3, 4, 5, 6].into_iter()) {
        Ok((parsed, rest)) => println!("{parsed:?} then {:?}", rest.collect::<Vec<_>>()),
        Err(rest) => println!("no, {:?}", rest.collect::<Vec<_>>()),
    }
    match pairs([1, 2, 9].into_iter()) {
        Ok((parsed, rest)) => println!("{parsed:?} then {:?}", rest.collect::<Vec<_>>()),
        Err(rest) => println!("no, {:?}", rest.collect::<Vec<_>>()),
    }
}
