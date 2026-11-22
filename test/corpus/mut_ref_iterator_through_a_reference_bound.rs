//@ library-differs
// A bound on a `&mut` of a type parameter, `for<'a> &'a mut C:
// IntoIterator`, given a `&mut` of an iterator, which std's impl for every
// iterator takes as it is: generic code has the `&mut C` in the box its
// caller makes of a `&mut` to an object, and steps the iterator in it,
// which the caller sees stepped (#2336). And a `T: IntoIterator` another
// bound names, `impl Is<T>`'s, whose value arrives as that one's, the
// `&mut` as concrete code has it (#966), and a `T: IntoIterator` given as
// it is, which steps what it makes. A `T: IntoIterator` and a `C:
// Iterator` given one, which hold it as a JS iterator, beside them.

struct Count(u32);

impl Iterator for Count {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.0 += 1;
        (self.0 < 6).then_some(self.0)
    }
}

fn two<C>(c: &mut C) -> Vec<u32>
where
    for<'a> &'a mut C: IntoIterator<Item = u32>,
{
    let mut v = Vec::new();
    for x in &mut *c {
        v.push(x);
        if v.len() == 2 {
            break;
        }
    }
    v
}

fn sum_rest<C>(c: &mut C) -> u32
where
    for<'a> &'a mut C: IntoIterator<Item = u32>,
{
    c.into_iter().sum()
}

fn first_of<C>(c: &mut C) -> Option<u32>
where
    for<'a> &'a mut C: IntoIterator<Item = u32>,
{
    let mut it = c.into_iter();
    it.next()
}

fn by_value<T: IntoIterator<Item = u32>>(t: T) -> Vec<u32> {
    t.into_iter().take(2).collect()
}

trait Is<T> {
    fn is(self) -> T;
}

impl<T> Is<T> for T {
    fn is(self) -> T {
        self
    }
}

fn by_another<T: IntoIterator<Item = u32>>(t: impl Is<T>) -> Vec<u32> {
    t.is().into_iter().take(2).collect()
}

fn kept_stepped<T: IntoIterator<Item = u32>>(t: T) -> (T::IntoIter, Option<u32>) {
    let mut it = t.into_iter();
    let first = it.next();
    (it, first)
}

fn two_stepped<C: Iterator<Item = u32>>(c: &mut C) -> Vec<u32>
where
    for<'a> &'a mut C: IntoIterator<Item = u32>,
{
    let mut v = two(c);
    v.extend(c.next());
    v
}

fn forwarded<I: Iterator<Item = u32>>(mut i: I) -> (Vec<u32>, Option<u32>) {
    let firsts = two(&mut i);
    (firsts, i.next())
}

fn main() {
    let mut r = 0..14u32;
    println!("{:?}", two(&mut r));
    println!("{:?} {:?}", first_of(&mut r), r);
    println!("{:?} {:?}", by_value(&mut r), r);
    println!("{:?} {:?}", by_another(&mut r), r);
    let (mut rest, first) = kept_stepped(&mut r);
    println!("{:?} {:?}", first, rest.next());
    println!("{:?} {:?}", two_stepped(&mut r), r);
    println!("{:?} {:?}", sum_rest(&mut r), r);

    let mut items = vec![7, 8, 9, 10, 11].into_iter();
    println!("{:?} {:?}", two(&mut items), items.as_slice());
    println!("{:?} {:?}", by_value(&mut items), items.as_slice());
    println!("{:?} {:?}", first_of(&mut items), items.as_slice());
    let mut more = vec![1, 2, 3].into_iter();
    println!("{:?} {:?}", by_another(&mut more), more.as_slice());
    let (rest, first) = kept_stepped(&mut more);
    println!("{:?} {:?}", first, rest.as_slice());

    let mut count = Count(0);
    println!("{:?} {}", two(&mut count), count.0);
    println!("{} {}", sum_rest(&mut count), count.0);
    let mut again = Count(1);
    println!("{:?} {}", by_another(&mut again), again.0);

    let mut tens = (0..6u32).map(|x| x * 10);
    println!("{:?} {:?}", two(&mut tens), tens.next());
    let mut copied = [1u32, 2, 3, 4].iter().copied();
    println!("{:?} {:?}", two(&mut copied), copied.next());

    println!("{:?}", forwarded(10..15u32));
    println!("{:?}", forwarded(Count(2)));

    let mut left = 100..103u32;
    let mut total = 0;
    while let Some(x) = first_of(&mut left) {
        total += x;
    }
    println!("{total} {left:?}");
}
