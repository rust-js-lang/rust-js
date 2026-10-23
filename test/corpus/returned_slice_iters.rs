// A slice's or a Vec's iterator a function of the crate's gives, its
// `$iter`, stepped from its back as one a variable holds is; and an
// `enumerate()` of one, or of one held, reversed, `$enumerate`, whose index
// from the back is where it is.

struct Bag(Vec<i32>);
impl Bag {
    fn items(&self) -> std::slice::Iter<'_, i32> {
        self.0.iter()
    }
    fn owned(self) -> std::vec::IntoIter<i32> {
        self.0.into_iter()
    }
}
impl<'a> IntoIterator for &'a Bag {
    type Item = &'a i32;
    type IntoIter = std::slice::Iter<'a, i32>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}
fn bag() -> Bag {
    Bag(vec![4, 1, 3, 2])
}
fn main() {
    let b = bag();
    println!("{:?}", b.items().map(|x| x * 2).collect::<Vec<_>>());
    println!("{:?}", b.items().filter(|&&x| x > 1).rev().collect::<Vec<_>>());
    println!(
        "{} {} {:?} {:?}",
        b.items().len(),
        b.items().count(),
        b.items().last(),
        b.items().nth(2)
    );
    println!(
        "{} {:?} {:?}",
        b.items().sum::<i32>(),
        b.items().max(),
        b.items().position(|&x| x == 3)
    );
    println!("{:?}", b.items().as_slice());
    println!(
        "{:?}",
        b.items().zip(b.items().skip(1)).map(|(a, c)| a - c).collect::<Vec<_>>()
    );
    println!("{:?}", b.items().enumerate().rev().take(2).collect::<Vec<_>>());
    println!("{:?}", b.items().copied().rev().step_by(2).collect::<Vec<_>>());
    println!("{:?}", (&b).into_iter().rev().skip(1).collect::<Vec<_>>());
    let mut it = b.items();
    it.next();
    println!("{:?} {:?}", it.as_slice(), it.rev().collect::<Vec<_>>());
    for (i, x) in (&b).into_iter().enumerate().rev() {
        print!("{i}:{x} ");
    }
    println!();
    println!("{:?}", bag().owned().rev().collect::<Vec<_>>());
    println!("{:?}", bag().owned().map(|x| x + 1).rev().collect::<Vec<_>>());
    let mut o = bag().owned();
    println!("{:?} {:?} {:?}", o.next_back(), o.next(), o.as_slice());
    println!("{:?}", b.items().rev().max_by_key(|&&x| x % 3));
    println!("{:?}", b.items().rposition(|&x| x < 3));
    let held = b.items();
    println!("{:?}", held.enumerate().rev().collect::<Vec<_>>());
}
