// A trait's associated type bound by `Iterator`, given to generic code that
// takes its iterator as it is, `advance(it: &mut I, ..)`, which a value it
// may write in place makes so: the impl's dictionary gives the type's
// `Iterator`, and its back's and length's where the trait bounds them so,
// as the trait declares them, std's adapters', a `Vec`'s and the crate's own
// alike, a generic associated type's too (#972).
#[derive(Clone)]
struct Countdown(u32, &'static str);

impl Iterator for Countdown {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        println!("{} from the back", self.1);
        self.next().map(|x| 10 + x)
    }
}

impl Drop for Countdown {
    fn drop(&mut self) {
        println!("drop {} at {}", self.0, self.1);
    }
}

trait Source {
    type Iter: DoubleEndedIterator<Item = u32>;
    fn items(&self) -> Self::Iter;
}

trait Container {
    type Iter<'a>: Iterator<Item = u32>
    where
        Self: 'a;
    fn iter(&self) -> Self::Iter<'_>;
}

struct Counted(u32);

impl Source for Counted {
    type Iter = Countdown;
    fn items(&self) -> Countdown {
        Countdown(self.0, "counted")
    }
}

struct Listed(Vec<u32>);

impl Source for Listed {
    type Iter = std::vec::IntoIter<u32>;
    fn items(&self) -> Self::Iter {
        self.0.clone().into_iter()
    }
}

impl Container for Listed {
    type Iter<'a> = std::iter::Copied<std::slice::Iter<'a, u32>>;
    fn iter(&self) -> Self::Iter<'_> {
        self.0.iter().copied()
    }
}

// An impl's own type parameter, given as it is: its dictionary's.
struct Kept<I>(I);

impl<I: DoubleEndedIterator<Item = u32> + Clone> Source for Kept<I> {
    type Iter = I;
    fn items(&self) -> I {
        self.0.clone()
    }
}

fn advance<I: Iterator>(it: &mut I, replacement: Option<I>) {
    match replacement {
        Some(other) => *it = other,
        None => {
            it.next();
        }
    }
}

fn from_the_back<I: DoubleEndedIterator>(it: &mut I, replacement: Option<I>) -> Option<I::Item> {
    if let Some(other) = replacement {
        *it = other;
    }
    it.next_back()
}

fn second<S: Source>(source: &S) -> Option<u32> {
    let mut items = source.items();
    advance(&mut items, None);
    items.next()
}

fn swapped<S: Source>(source: &S, other: &S) -> Vec<u32> {
    let mut items = source.items();
    advance(&mut items, Some(other.items()));
    items.collect()
}

fn last<S: Source>(source: &S) -> Option<u32> {
    let mut items = source.items();
    from_the_back(&mut items, None)
}

fn second_borrowed<C: Container>(container: &C) -> Option<u32> {
    let mut items = container.iter();
    advance(&mut items, None);
    items.next()
}

fn main() {
    println!("{:?}", second(&Counted(3)));
    println!("{:?}", second(&Listed(vec![4, 5, 6])));
    println!("{:?}", swapped(&Counted(2), &Counted(4)));
    println!("{:?}", swapped(&Listed(vec![1]), &Listed(vec![7, 8])));
    println!("{:?}", last(&Counted(3)));
    println!("{:?}", last(&Listed(vec![4, 5, 6])));
    println!("{:?}", second_borrowed(&Listed(vec![9, 8, 7])));
    println!("{:?}", second(&Kept(Countdown(4, "kept"))));
    println!("{:?}", last(&Kept(Countdown(2, "kept"))));
}
