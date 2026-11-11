//@ library-differs
// An adapter compiled as its meaning alone, `cloned()`'s, whose `{:?}` the
// crate asks, so it's std's struct where it's held (#1401), cloned by
// generic code given it as an `I: Clone`, as itertools' `Product` clones
// the iterator it's given: std's own `Clone` of the struct, not the
// recognizers' of a JS iterator (#1409). And through a trait's default of
// a blanket impl, as `cartesian_product` is called, where only a bound of
// a projection of a holder of it, `<Held<I> as Iterator>::Item: Debug`,
// names the adapter, as itertools' `assert_equal` of a `PutBack` does;
// and given through `IntoIterator::into_iter`, as `iproduct!` gives it.
use std::fmt::Debug;

#[derive(Clone, Debug)]
struct Wrap<I> {
    iter: I,
}

struct Held<I> {
    iter: I,
}

impl<I: Iterator> Iterator for Held<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        self.iter.next()
    }
}

fn twice<J: Iterator<Item = i32> + Clone>(j: J) -> i32 {
    j.clone().sum::<i32>() + j.sum::<i32>()
}

fn shown<I: IntoIterator>(items: I) -> usize
where
    I::Item: Debug,
{
    items.into_iter().map(|item| format!("{item:?}").len()).sum()
}

trait Tools: Iterator {
    fn paired<J>(self, other: J) -> i32
    where
        Self: Sized + Iterator<Item = i32>,
        J: IntoIterator<Item = i32>,
        J::IntoIter: Clone,
    {
        let other = other.into_iter();
        self.map(|a| a * other.clone().sum::<i32>()).sum()
    }
}

impl<T: Iterator + ?Sized> Tools for T {}

fn main() {
    let w = Wrap {
        iter: [1, 2].iter().cloned(),
    };
    println!("{w:?}");
    println!("{}", twice(w.iter.clone()));
    println!("{}", twice([1, 2, 3].iter().cloned()));
    println!("{}", twice([1, 2, 3].iter().copied().filter(|x| x % 2 == 1)));
    let xs = [10, 20, 30];
    println!("{}", shown(Held { iter: xs.iter().cloned() }));
    println!("{}", [0, 1, 2].iter().cloned().paired([0, 1, 2].iter().cloned()));
    println!(
        "{}",
        Tools::paired(
            IntoIterator::into_iter([0, 1, 2].iter().cloned()),
            IntoIterator::into_iter([0, 1, 2].iter().cloned()),
        )
    );
}
