//@ library-differs
// An iterator's own `last`, taking nothing of its own type parameters but
// the layout of its impl's `I::Item`, which its collect asks: given by the
// caller of the type it knows (#840).
struct Wrap<I>(I);

impl<I: Iterator> Iterator for Wrap<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }

    fn last(self) -> Option<I::Item> {
        let all: Vec<I::Item> = self.0.collect();
        println!("own last of {}", all.len());
        all.into_iter().last()
    }
}

fn main() {
    println!("{:?}", Wrap(vec![1, 2, 3].into_iter()).last());
    println!("{:?}", Wrap(vec![1u8, 2].into_iter()).map(|x| x * 2).last());
    println!("{:?}", Wrap("abc".chars()).skip(1).last());
}
