// Generic code's `t.into_iter()` of a `T: IntoIterator` whose `T::IntoIter`
// a bound says is a `DoubleEndedIterator` or an `ExactSizeIterator`, given
// the crate's own iterator: stepped from its back, folded from there by its
// own `rfold` where it writes one, asked its length, and given on to code
// that asks them, as std does. Its caller made it a JS
// iterator knowing neither, so `rev()` threw `it.back is not a function`
// and `len()` `it.len is not a function` (#1604).
struct Up(u32, u32);

impl Iterator for Up {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 < self.1 {
            self.0 += 1;
            Some(self.0)
        } else {
            None
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.1 - self.0) as usize;
        (n, Some(n))
    }
}

impl DoubleEndedIterator for Up {
    fn next_back(&mut self) -> Option<u32> {
        if self.0 < self.1 {
            self.1 -= 1;
            Some(self.1 + 1)
        } else {
            None
        }
    }
}

impl ExactSizeIterator for Up {}

// One whose own `rfold` generic code's fold from its back reaches, as std's
// does, and which takes it: dropped there, once.
struct Folded(u32);

impl Drop for Folded {
    fn drop(&mut self) {
        println!("drop folded {}", self.0);
    }
}

impl Iterator for Folded {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        (self.0 > 0).then(|| {
            self.0 -= 1;
            self.0
        })
    }
}

impl DoubleEndedIterator for Folded {
    fn next_back(&mut self) -> Option<u32> {
        self.next()
    }

    fn rfold<B, F: FnMut(B, u32) -> B>(self, init: B, f: F) -> B {
        println!("own rfold of {}", self.0);
        (0..self.0).fold(init, f)
    }
}

fn rev_first<T: IntoIterator>(t: T) -> Option<T::Item>
where
    T::IntoIter: DoubleEndedIterator,
{
    t.into_iter().rev().next()
}

fn len_of<T: IntoIterator>(t: T) -> usize
where
    T::IntoIter: ExactSizeIterator,
{
    t.into_iter().len()
}

fn doubled_from_the_back<T: IntoIterator<Item = u32>>(t: T) -> Vec<u32>
where
    T::IntoIter: DoubleEndedIterator,
{
    t.into_iter().map(|x| x * 2).rev().collect()
}

fn counted_from_the_back<T: IntoIterator>(t: T) -> usize
where
    T::IntoIter: DoubleEndedIterator,
{
    t.into_iter().rev().count()
}

fn last_from_the_back<T: IntoIterator>(t: T) -> Option<T::Item>
where
    T::IntoIter: DoubleEndedIterator,
{
    t.into_iter().rev().last()
}

fn summed_from_the_back<T: IntoIterator<Item = u32>>(t: T) -> u32
where
    T::IntoIter: DoubleEndedIterator,
{
    t.into_iter().rfold(0, |sum, x| sum * 10 + x)
}

fn reversed_sum<T: IntoIterator<Item = u32>>(t: T) -> u32
where
    T::IntoIter: DoubleEndedIterator,
{
    t.into_iter().rev().fold(0, |sum, x| sum * 10 + x)
}

// A `T` another bound names, its caller's own value given as it is (#966),
// folded from its back by its dictionaries.
trait Is<T> {
    fn is(self) -> T;
}

impl<T> Is<T> for T {
    fn is(self) -> T {
        self
    }
}

fn summed_as_it_is<T: IntoIterator<Item = u32>>(t: impl Is<T>) -> u32
where
    T::IntoIter: DoubleEndedIterator,
{
    t.is().into_iter().rfold(0, |sum, x| sum * 10 + x)
}

fn backs<I: DoubleEndedIterator>(mut it: I) -> Option<I::Item> {
    it.next_back()
}

fn lens<I: ExactSizeIterator>(it: I) -> usize {
    it.len()
}

fn given_on<T: IntoIterator>(t: T) -> Option<T::Item>
where
    T::IntoIter: DoubleEndedIterator,
{
    backs(t.into_iter())
}

fn length_given_on<T: IntoIterator>(t: T) -> usize
where
    T::IntoIter: ExactSizeIterator,
{
    lens(t.into_iter())
}

fn main() {
    println!("{:?}", rev_first(Up(0, 3)));
    println!("{}", len_of(Up(0, 3)));
    println!("{:?}", doubled_from_the_back(Up(0, 3)));
    println!("{}", counted_from_the_back(Up(1, 5)));
    println!("{:?}", last_from_the_back(Up(0, 3)));
    println!("{:?}", given_on(Up(2, 6)));
    println!("{}", length_given_on(Up(2, 6)));
    println!("{}", summed_from_the_back(Folded(3)));
    println!("{}", summed_from_the_back(Up(0, 3)));
    println!("{}", reversed_sum(Folded(4)));
    println!("{}", summed_as_it_is(Up(0, 3)));
    println!("{}", summed_as_it_is(Folded(2)));
    println!("{:?}", rev_first(vec![1, 2]));
    println!("{}", len_of(vec![1, 2]));
    println!("{:?}", doubled_from_the_back(vec![1, 2, 3]));
    println!("{}", counted_from_the_back(vec![4, 5]));
    println!("{:?}", last_from_the_back(vec![7, 8, 9]));
    println!("{:?}", given_on(vec![3, 4]));
    println!("{}", summed_from_the_back(vec![1, 2, 3]));
    println!("{}", summed_as_it_is(vec![4, 5]));
    println!("{}", length_given_on(vec![3, 4, 5]));
}
