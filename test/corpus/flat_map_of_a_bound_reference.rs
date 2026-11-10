//@ library-differs
// `flat_map(|_| r)` of a reference whose `IntoIterator` is a bound of its
// function's: std's flatten asks its items' constant size, which an
// array's reference has and another's doesn't, so its `size_hint` differs
// by what the caller gives (#1273).
struct Word(Vec<char>);

impl<'a> IntoIterator for &'a Word {
    type Item = &'a char;
    type IntoIter = std::slice::Iter<'a, char>;
    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

fn twice<'a, I>(r: &'a I) -> (Vec<&'a char>, (usize, Option<usize>))
where
    &'a I: IntoIterator<Item = &'a char>,
{
    let mut it = [0, 1].iter().flat_map(|_| r);
    let hint = it.size_hint();
    it.next();
    (it.collect(), hint)
}

fn hints<'a, I>(r: &'a I) -> Vec<(usize, Option<usize>)>
where
    &'a I: IntoIterator<Item = &'a char>,
{
    let mut it = [0, 1, 2].into_iter().flat_map(|_| r);
    let mut seen = vec![it.size_hint()];
    while it.next().is_some() {
        seen.push(it.size_hint());
    }
    seen
}

fn mutable<'a, I>(r: &'a mut I) -> Vec<(usize, Option<usize>)>
where
    &'a mut I: IntoIterator<Item = &'a mut u8>,
{
    let mut it = Some(r).into_iter().flat_map(|r| r);
    let mut seen = vec![it.size_hint()];
    while let Some(x) = it.next() {
        *x += 1;
        seen.push(it.size_hint());
    }
    seen
}

// Generic code giving its own caller's reference on: the constant size
// asked of its caller.
fn forwarded<'a, J>(r: &'a J) -> (Vec<&'a char>, (usize, Option<usize>))
where
    &'a J: IntoIterator<Item = &'a char>,
{
    twice(r)
}

fn main() {
    let w = Word(vec!['a', 'b']);
    println!("{:?}", twice(&w));
    println!("{:?}", hints(&w));
    let a = ['x', 'y', 'z'];
    println!("{:?}", twice(&a));
    println!("{:?}", hints(&a));
    let v = vec!['p'];
    println!("{:?}", twice(&v));
    println!("{:?}", hints(&v));
    println!("{:?}", forwarded(&a));
    println!("{:?}", forwarded(&w));
    let mut b = [1u8, 2];
    println!("{:?} {:?}", mutable(&mut b), b);
    let mut c = vec![5u8];
    println!("{:?} {:?}", mutable(&mut c), c);
}
