// What an `IntoIterator` makes, held as it is where its function bounds it
// by `Clone`: of a type parameter's, and of an associated type's, as
// itertools' `multi_cartesian_product` holds it.

#[derive(Clone)]
struct Twice<I> {
    iter: I,
    orig: I,
}

fn twice<T: IntoIterator>(items: T) -> Twice<T::IntoIter>
where
    T::IntoIter: Clone,
{
    let iter = items.into_iter();
    Twice {
        iter: iter.clone(),
        orig: iter,
    }
}

impl<I: Iterator + Clone> Iterator for Twice<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.iter.next().or_else(|| {
            self.iter = self.orig.clone();
            self.iter.next()
        })
    }
}

fn twices<H>(all: H) -> Vec<Twice<<H::Item as IntoIterator>::IntoIter>>
where
    H: Iterator,
    H::Item: IntoIterator,
    <H::Item as IntoIterator>::IntoIter: Clone,
{
    all.map(|items| {
        let iter = items.into_iter();
        Twice {
            iter: iter.clone(),
            orig: iter,
        }
    })
    .collect()
}

fn main() {
    let a: Vec<u32> = twice(0..3).take(5).collect();
    let b: Vec<u32> = twice(vec![7, 8]).take(3).collect();
    let c: String = twice("ab".chars()).take(3).collect();
    println!("{a:?} {b:?} {c}");
    for t in twices(vec![0..2, 5..6].into_iter()) {
        let firsts: Vec<u32> = t.take(3).collect();
        println!("{firsts:?}");
    }
    for t in twices(vec![vec!['a', 'b'], vec!['c']].into_iter()) {
        let firsts: String = t.take(3).collect();
        println!("{firsts}");
    }
}
