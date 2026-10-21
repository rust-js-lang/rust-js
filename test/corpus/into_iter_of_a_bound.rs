// An `IntoIterator` given to generic code that holds what it makes as it is
// (ADR 0552), of a type only bounds know: an iterator, `J::IntoIter`, whose
// `IntoIterator` is std's for every iterator, itself; and an impl's bound,
// which its method's result holds, given to the impl's dictionary, as
// itertools' `put_back` and `Zip::from` are.
pub struct PutBack<I: Iterator> {
    top: Option<I::Item>,
    iter: I,
}

pub fn put_back<I: IntoIterator>(iterable: I) -> PutBack<I::IntoIter> {
    PutBack {
        top: None,
        iter: iterable.into_iter(),
    }
}

impl<I: Iterator> PutBack<I> {
    pub fn with_value(mut self, value: I::Item) -> Self {
        self.top = Some(value);
        self
    }
}

impl<I: Iterator> Iterator for PutBack<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        self.top.take().or_else(|| self.iter.next())
    }
}

pub fn rest<J: IntoIterator>(j: J) -> Vec<J::Item> {
    let mut j = j.into_iter();
    match j.next() {
        Some(first) => put_back(j).with_value(first).collect(),
        None => Vec::new(),
    }
}

pub struct Pair<T> {
    t: T,
}

impl<A: IntoIterator, B: IntoIterator> From<(A, B)> for Pair<(A::IntoIter, B::IntoIter)> {
    fn from((a, b): (A, B)) -> Self {
        Pair {
            t: (a.into_iter(), b.into_iter()),
        }
    }
}

impl<A: Iterator, B: Iterator> Iterator for Pair<(A, B)> {
    type Item = (A::Item, B::Item);
    fn next(&mut self) -> Option<Self::Item> {
        Some((self.t.0.next()?, self.t.1.next()?))
    }
}

pub fn paired<T, U>(t: U) -> Pair<T>
where
    Pair<T>: From<U> + Iterator,
{
    Pair::from(t)
}

fn main() {
    println!("{:?}", rest(vec![1, 2, 3]));
    println!("{:?}", rest(Vec::<u8>::new()));
    let pairs: Vec<(i32, char)> = Pair::from((vec![1, 2, 3], "ab".chars())).collect();
    println!("{pairs:?}");
    let zipped: Vec<(u8, char)> = paired((vec![4, 5], vec!['x', 'y', 'z'])).collect();
    println!("{zipped:?}");
}
