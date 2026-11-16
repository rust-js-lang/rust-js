// A `Vec`'s or a slice's iterator stepped, then lent as a `&mut dyn
// DoubleEndedIterator`, is the one stepped, where it is: never read as an
// array's items from their start (#1985). Only stepped, it isn't the
// array's own iterator, `a.values()`, which a `$iter` hasn't; looped over,
// it isn't an index loop over `a.length`, which a `$iter` hasn't either,
// and which ran no step. Of items that drop, the lender stepped after;
// stepped by `while let`; and looped over by `for`.
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds() -> Vec<D> {
    (1..=5).map(D).collect()
}

fn stepped_by_next() {
    let mut a = ds().into_iter();
    println!("a next {:?}", a.next().map(|x| x.0));
    let b: &mut dyn DoubleEndedIterator<Item = D> = &mut a;
    println!("b next {:?}", b.next().map(|x| x.0));
    println!("b next {:?}", b.next().map(|x| x.0));
    println!("a next_back {:?}", a.next_back().map(|x| x.0));
    for x in a {
        println!("a rest {}", x.0);
    }
}

fn stepped_while_some() {
    let mut a = vec![1u32, 2, 3, 4].into_iter();
    a.next();
    let b: &mut dyn DoubleEndedIterator<Item = u32> = &mut a;
    let mut sum = 0;
    while let Some(x) = b.next() {
        sum += x;
    }
    println!("sum {sum}");
}

fn looped() {
    let items = [10u32, 20, 30, 40];
    let mut a = items.iter();
    a.next();
    let b: &mut dyn DoubleEndedIterator<Item = &u32> = &mut a;
    for x in b {
        println!("b {x}");
    }
    println!("a next {:?}", a.next());
}

fn main() {
    stepped_by_next();
    stepped_while_some();
    looped();
}
