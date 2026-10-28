// A `copied()` of an iterator generic code is given, bound by
// `ExactSizeIterator`, knows its length, as std's `Copied` asks its
// source's (#387). Given as it is, by a bound that passes it dictionaries,
// the JS iterator made of it is given its `len`: of the crate's own
// iterator, whose `$given` is a JS iterator stepped by its dictionary, and
// of a slice's. Given no dictionaries, it's the JS iterator its caller made.
// Of items copied as it steps, structs something changes, a map of it that
// knows its length too. Of a `&mut` to one, lent, its lender's stepped on
// with it, and after it.

#[derive(Clone)]
struct Refs {
    items: &'static [u32],
    at: usize,
}

impl Iterator for Refs {
    type Item = &'static u32;

    fn next(&mut self) -> Option<&'static u32> {
        let item = self.items.get(self.at)?;
        self.at += 1;
        Some(item)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.items.len() - self.at;
        (n, Some(n))
    }
}

impl ExactSizeIterator for Refs {}

static ITEMS: [u32; 3] = [1, 2, 3];

fn refs(at: usize) -> Refs {
    Refs { items: &ITEMS, at }
}

fn sized<'a, I: ExactSizeIterator<Item = &'a u32> + Clone>(it: I) -> usize {
    let mut copied = it.copied();
    copied.next();
    copied.len()
}

// Its length read again after each step, and a clone's apart.
fn counted<'a, I: ExactSizeIterator<Item = &'a u32> + Clone>(it: I) -> Vec<(u32, usize)> {
    let mut copied = it.copied();
    let again = copied.clone();
    let mut seen = Vec::new();
    while let Some(item) = copied.next() {
        seen.push((item, copied.len()));
    }
    seen.push((0, again.len()));
    seen
}

#[derive(Clone, Copy, Debug)]
struct Point {
    x: u32,
}

fn moved<'a, I: ExactSizeIterator<Item = &'a Point> + Clone>(it: I) -> (Point, usize, Vec<Point>) {
    let mut copied = it.copied();
    let mut first = copied.next().unwrap();
    first.x += 10;
    (first, copied.len(), copied.collect())
}

fn moved_plain<'a, I: ExactSizeIterator<Item = &'a Point>>(it: I) -> (Point, usize, Vec<Point>) {
    let mut copied = it.copied();
    let mut first = copied.next().unwrap();
    first.x += 10;
    (first, copied.len(), copied.collect())
}

fn lent<'a, I: ExactSizeIterator<Item = &'a u32> + Clone>(mut it: I) -> (usize, Option<u32>, usize) {
    let mut copied = (&mut it).copied();
    copied.next();
    let n = copied.len();
    let next = copied.next();
    (n, next, it.len())
}

// What stops early, `take(1)`, leaves its lender to step on.
fn lent_plain<'a, I: ExactSizeIterator<Item = &'a u32>>(mut it: I) -> (usize, Vec<u32>, Option<&'a u32>) {
    let copied = (&mut it).copied();
    let n = copied.len();
    let first = copied.take(1).collect();
    (n, first, it.next())
}

fn main() {
    println!("{} {}", sized(refs(0)), sized(ITEMS.iter()));
    println!("{:?}", counted(refs(1)));
    let points = vec![Point { x: 1 }, Point { x: 2 }, Point { x: 3 }];
    println!("{:?}", moved(points.iter()));
    println!("{:?}", moved_plain(points.iter()));
    println!("{points:?}");
    println!("{:?} {:?}", lent(refs(0)), lent_plain(ITEMS.iter().rev()));
}
