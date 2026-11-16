// A trait method's `impl Iterator` within what it gives, a tuple's field or
// an `Option`'s payload: generic code steps each as the JS iterator the
// impl's dictionary makes of it, as of one given alone (#2005, ADR 0061,
// #1292). A range, which is no JS iterator as it's made, the crate's own
// iterator, one stepped from its back, a nested tuple's, and items and a
// field with a destructor, each dropped as Rust drops them.
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Count(u32);

impl Iterator for Count {
    type Item = u32;

    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 {
            None
        } else {
            self.0 -= 1;
            Some(self.0)
        }
    }
}

trait Source {
    fn tup(&self) -> (impl Iterator<Item = u32>, u8);
    fn back(&self) -> (u8, impl DoubleEndedIterator<Item = u32>);
    fn nested(&self) -> ((impl Iterator<Item = u32>, u8), impl Iterator<Item = u32>);
    fn maybe(&self, some: bool) -> Option<impl Iterator<Item = u32>>;
    fn louds(&self) -> (impl Iterator<Item = Loud>, Loud);
}

struct A;

impl Source for A {
    fn tup(&self) -> (impl Iterator<Item = u32>, u8) {
        (0..3, 7)
    }

    fn back(&self) -> (u8, impl DoubleEndedIterator<Item = u32>) {
        (1, 10..13)
    }

    fn nested(&self) -> ((impl Iterator<Item = u32>, u8), impl Iterator<Item = u32>) {
        ((Count(3), 2), vec![5, 6].into_iter())
    }

    fn maybe(&self, some: bool) -> Option<impl Iterator<Item = u32>> {
        if some { Some(4..6) } else { None }
    }

    fn louds(&self) -> (impl Iterator<Item = Loud>, Loud) {
        (vec![Loud(1), Loud(2), Loud(3)].into_iter(), Loud(9))
    }
}

fn read<T: Source>(t: &T) {
    let (it, k) = t.tup();
    println!("tup {} {k}", it.sum::<u32>());
    let (j, mut b) = t.back();
    let last = b.next_back();
    println!("back {j} {last:?} {:?}", b.collect::<Vec<_>>());
    let ((c, n), v) = t.nested();
    println!("nested {:?} {n} {:?}", c.collect::<Vec<_>>(), v.collect::<Vec<_>>());
    println!("maybe {:?} {:?}", t.maybe(true).map(|it| it.sum::<u32>()), t.maybe(false).map(|it| it.count()));
    let (mut louds, held) = t.louds();
    let first = louds.next().map(|l| l.0);
    println!("first {first:?}");
    drop(louds);
    println!("held {}", held.0);
}

fn main() {
    read(&A);
}
