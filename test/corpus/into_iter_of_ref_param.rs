// A bound on a reference to a type parameter, `for<'a> &'a mut C:
// IntoIterator`, as either's `iter_mut` and generic collection code write
// it: the caller's `into_iter` of a `&C` or a `&mut C`, std's or the
// crate's, with `C` kept.

use std::collections::BTreeMap;

fn total<C>(c: &C) -> u32
where
    for<'a> &'a C: IntoIterator<Item = &'a u32>,
{
    c.into_iter().sum()
}

fn double_all<C>(c: &mut C)
where
    for<'a> &'a mut C: IntoIterator<Item = &'a mut u32>,
{
    for x in c {
        *x *= 2;
    }
}

fn double_then_total<C>(c: &mut C) -> u32
where
    for<'a> &'a C: IntoIterator<Item = &'a u32>,
    for<'a> &'a mut C: IntoIterator<Item = &'a mut u32>,
{
    double_all(c);
    double_all(c);
    total(c)
}

fn bump_values<M>(m: &mut M)
where
    for<'a> &'a mut M: IntoIterator<Item = (&'a String, &'a mut u32)>,
{
    for (key, value) in m {
        *value += key.len() as u32;
    }
}

// `for x in &mut it` of an `I: Iterator`, which std's impl gives: no bound.
fn sum_past_three<I: Iterator<Item = u32>>(mut it: I) -> (u32, usize) {
    let mut n = 0;
    for x in &mut it {
        n += x;
        if n > 3 {
            break;
        }
    }
    (n, it.count())
}

struct Bag {
    items: Vec<u32>,
}

impl<'a> IntoIterator for &'a Bag {
    type Item = &'a u32;
    type IntoIter = std::slice::Iter<'a, u32>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<'a> IntoIterator for &'a mut Bag {
    type Item = &'a mut u32;
    type IntoIter = std::slice::IterMut<'a, u32>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter_mut()
    }
}

fn main() {
    let mut v = vec![1, 2, 3];
    double_all(&mut v);
    let mut a = [4, 5];
    double_all(&mut a);
    println!("{v:?} {a:?} {} {}", total(&v), total(&a));

    let mut some = Some(7);
    double_all(&mut some);
    let mut none: Option<u32> = None;
    double_all(&mut none);
    println!("{some:?} {none:?} {}", total(&some));

    let mut bag = Bag { items: vec![1, 10] };
    println!("{} {:?}", double_then_total(&mut bag), bag.items);

    let mut m = BTreeMap::from([(String::from("ab"), 1), (String::from("c"), 2)]);
    bump_values(&mut m);
    println!("{m:?}");

    println!("{:?}", sum_past_three([1, 2, 3, 4, 5].into_iter()));
}
