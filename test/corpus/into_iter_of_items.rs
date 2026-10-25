// Generic code given items as an associated type bound by `IntoIterator`,
// `I::Item: IntoIterator`, holds each as it is with its dictionary, which
// makes and steps its iterator (ADR 0552, #278): `Option`s, whose JS is
// `T | undefined`, `Vec`s, arrays and the crate's own iterators, iterated
// by a `for` loop, given on to a function that takes any `IntoIterator`,
// summed in a closure and flattened.
#[derive(Clone)]
struct Countdown(i32);

impl Iterator for Countdown {
    type Item = i32;
    fn next(&mut self) -> Option<i32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
}

fn total<T: IntoIterator<Item = i32>>(items: T) -> i32 {
    let mut sum = 0;
    for x in items {
        sum += x;
    }
    sum
}

fn looped<I: Iterator>(items: I) -> i32
where
    I::Item: IntoIterator<Item = i32>,
{
    let mut sum = 0;
    for item in items {
        for x in item {
            sum += x;
        }
    }
    sum
}

fn given_on<I: Iterator>(items: I) -> i32
where
    I::Item: IntoIterator<Item = i32>,
{
    let mut sum = 0;
    for item in items {
        sum += total(item);
    }
    sum
}

fn summed<I: Iterator>(items: I) -> Vec<i32>
where
    I::Item: IntoIterator<Item = i32>,
{
    items.map(|item| item.into_iter().sum()).collect()
}

fn flat<I: Iterator>(items: I) -> i32
where
    I::Item: IntoIterator<Item = i32>,
{
    items.flatten().sum()
}

fn main() {
    let options = || vec![Some(1), None, Some(3)].into_iter();
    println!("{} {} {:?} {}", looped(options()), given_on(options()), summed(options()), flat(options()));
    let rows = || vec![vec![1, 2], vec![], vec![3]].into_iter();
    println!("{} {} {:?} {}", looped(rows()), given_on(rows()), summed(rows()), flat(rows()));
    let pairs = || [[1, 2], [3, 4]].into_iter();
    println!("{} {} {:?} {}", looped(pairs()), given_on(pairs()), summed(pairs()), flat(pairs()));
    let counts = || vec![Countdown(2), Countdown(3)].into_iter();
    println!("{} {} {:?} {}", looped(counts()), given_on(counts()), summed(counts()), flat(counts()));
}
