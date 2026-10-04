// A type of the crate's that's a collection: looped over by its own
// `IntoIterator`, built by `collect()` through its `FromIterator`, grown by
// its `Extend`, and added up by its `Sum`, as a domain model's are.

use std::iter::Sum;
use std::ops::Add;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Money {
    cents: i64,
}

impl Add for Money {
    type Output = Money;
    fn add(self, other: Money) -> Money {
        Money { cents: self.cents + other.cents }
    }
}

impl Sum for Money {
    fn sum<I: Iterator<Item = Money>>(items: I) -> Money {
        items.fold(Money::default(), |a, b| a + b)
    }
}

impl<'a> Sum<&'a Money> for Money {
    fn sum<I: Iterator<Item = &'a Money>>(items: I) -> Money {
        items.copied().sum()
    }
}

#[derive(Debug, Default)]
struct Cart {
    lines: Vec<(String, Money)>,
}

impl FromIterator<(String, Money)> for Cart {
    fn from_iter<I: IntoIterator<Item = (String, Money)>>(items: I) -> Cart {
        Cart { lines: items.into_iter().collect() }
    }
}

impl Extend<(String, Money)> for Cart {
    fn extend<I: IntoIterator<Item = (String, Money)>>(&mut self, items: I) {
        for line in items {
            self.lines.push(line);
        }
    }
}

impl IntoIterator for Cart {
    type Item = (String, Money);
    type IntoIter = std::vec::IntoIter<(String, Money)>;
    fn into_iter(self) -> Self::IntoIter {
        self.lines.into_iter()
    }
}

impl<'a> IntoIterator for &'a Cart {
    type Item = &'a (String, Money);
    type IntoIter = std::slice::Iter<'a, (String, Money)>;
    fn into_iter(self) -> Self::IntoIter {
        self.lines.iter()
    }
}

fn main() {
    let prices = [Money { cents: 250 }, Money { cents: 199 }];
    let owned: Money = prices.iter().copied().sum();
    let borrowed: Money = prices.iter().sum();
    println!("{:?} {:?}", owned, borrowed);

    let mut cart: Cart = vec![("tea", 250), ("cake", 199)]
        .into_iter()
        .map(|(name, cents)| (name.to_string(), Money { cents }))
        .collect();
    cart.extend([("jam".to_string(), Money { cents: 300 })]);
    for (name, price) in &cart {
        println!("{name}: {}", price.cents);
    }
    let total: Money = cart.lines.iter().map(|(_, m)| *m).sum();
    println!("{:?} {}", total, (&cart).into_iter().count());
    let names: Vec<String> = cart.into_iter().map(|(name, _)| name).collect();
    println!("{:?}", names);
}
