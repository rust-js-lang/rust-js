//@ library-differs
// An impl's own type parameter, `U` of `impl<U> Pair for Keeper<U>`, is a
// dictionary's to decide, never what a naming of the trait's method gives
// its own `T` (#1009): so its counted `Rc<U>` may be any `Rc`, the
// `Rc<Plain>` given it through generic code too, whose type drops nothing.
use std::fmt::Debug;
use std::marker::PhantomData;
use std::rc::Rc;

trait Pair {
    type Item;
    fn both<T: Debug>(&self, value: T, mine: &Rc<Self::Item>) -> usize;
}

struct Keeper<U>(PhantomData<U>);

impl<U: Debug> Pair for Keeper<U> {
    type Item = U;

    fn both<T: Debug>(&self, value: T, mine: &Rc<U>) -> usize {
        println!("{mine:?} {value:?}");
        Rc::strong_count(mine)
    }
}

fn through<P: Pair<Item = Plain>>(pair: &P, mine: &Rc<Plain>) -> usize {
    pair.both(7u8, mine)
}

#[derive(Debug)]
struct Plain(u32);

fn main() {
    let loud = Rc::new(Plain(3));
    let again = Rc::clone(&loud);
    println!("{}", through(&Keeper(PhantomData), &loud));
    drop(loud);
    println!("{}", Rc::strong_count(&again));
}
