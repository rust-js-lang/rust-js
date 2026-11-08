// An `IntoIterator` type parameter another bound names, `impl Is<I>`'s,
// arrives as that one's value, its caller's own: held as it is, made an
// iterator by its dictionary's `into_iter`, in a loop, a sum and a
// collection's `extend` alike; a `&mut` collection's as generic code holds
// it, in the box a `&mut` to an object is, its items' handles written in
// place (#966).
struct Quiet(u32);

impl Iterator for Quiet {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        self.0 = self.0.checked_sub(1)?;
        Some(self.0)
    }
}

#[derive(PartialEq)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Is<T> {
    fn is(self) -> T;
}

impl<T> Is<T> for T {
    fn is(self) -> T {
        self
    }
}

fn total<'a, I: IntoIterator<Item = &'a u32>>(items: impl Is<I>) -> u32 {
    let items: I = items.is();
    items.into_iter().sum()
}

fn bump<'a, I: IntoIterator<Item = &'a mut u32>>(items: impl Is<I>) {
    let items: I = items.is();
    for x in items {
        *x += 1;
    }
}

fn count<I: IntoIterator>(items: impl Is<I>) -> usize {
    let items: I = items.is();
    let mut n = 0;
    for _ in items {
        n += 1;
    }
    n
}

fn fill<C: Extend<u32>, I: IntoIterator<Item = u32>>(c: &mut C, items: impl Is<I>) {
    let items: I = items.is();
    c.extend(items);
}

fn equal<I: IntoIterator, J: IntoIterator>(a: impl Is<I>, b: impl Is<J>) -> bool
where
    I::Item: PartialEq<J::Item>,
{
    let a: I = a.is();
    let b: J = b.is();
    a.into_iter().eq(b)
}

fn main() {
    let mut v = vec![1u32, 2, 3];
    println!("{} {}", total(&v), total(&[4u32, 5]));
    bump(&mut v);
    bump(v.iter_mut().skip(1));
    println!("{v:?}");
    println!("{} {} {}", count(Quiet(4)), count(Some(7)), count(vec!["a", "b"]));
    let mut out: Vec<u32> = Vec::new();
    fill(&mut out, Quiet(3));
    fill(&mut out, [9, 8]);
    println!("{out:?}");
    println!("{}", equal(vec![Loud(1), Loud(2)], vec![Loud(1), Loud(3)]));
}
