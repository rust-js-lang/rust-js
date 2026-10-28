// A clone of a `copied()` of the crate's own iterator, and of one generic
// code is given, clones what it copies, not the JS iterator made of it
// (#384): each clone steps on from where it was made, apart from the one
// it was cloned from. Of std's iterators, test/lowering-iterators.test.ts's
// "a clone of a generic iterator's copied() is its clone".

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
}

static ITEMS: [u32; 3] = [1, 2, 3];

fn refs() -> Refs {
    Refs { items: &ITEMS, at: 0 }
}

fn apart<'a, I: Iterator<Item = &'a u32> + Clone>(it: I) -> (u32, u32) {
    let mut copied = it.copied();
    copied.next();
    let again = copied.clone();
    (copied.sum(), again.max().unwrap_or(0))
}

// A clone of a clone, stepped between.
fn twice_apart<'a, I: Iterator<Item = &'a u32> + Clone>(it: I) -> (Vec<u32>, Vec<u32>, Vec<u32>) {
    let mut copied = it.copied();
    copied.next();
    let mut once = copied.clone();
    once.next();
    let twice = once.clone();
    (copied.collect(), once.collect(), twice.collect())
}

// Cloned where it's given on, as its own type parameter.
fn both<J: Iterator<Item = u32> + Clone>(j: J) -> (Vec<u32>, Vec<u32>) {
    let k = j.clone();
    (j.collect(), k.collect())
}

fn passed_on<'a, I: Iterator<Item = &'a u32> + Clone>(it: I) -> (Vec<u32>, Vec<u32>) {
    let mut copied = it.copied();
    copied.next();
    both(copied)
}

#[derive(Clone)]
struct Holder {
    it: std::iter::Copied<Refs>,
}

fn main() {
    println!("{:?}", apart(refs()));
    let once = twice_apart(refs());
    let given = passed_on(refs());
    let copied = both(refs().copied());
    println!("{once:?} {given:?} {copied:?}");

    let mut copied = refs().copied();
    copied.next();
    let again = copied.clone();
    println!("{:?} {:?}", copied.collect::<Vec<_>>(), again.collect::<Vec<_>>());

    let mut held = Holder { it: refs().copied() };
    held.it.next();
    let other = held.clone();
    held.it.next();
    println!("{:?} {:?}", held.it.collect::<Vec<_>>(), other.it.collect::<Vec<_>>());
}
