//@ library-differs
// A trait method's result holding both a `&mut` to an associated type,
// which generic code holds as its box, and an `impl Iterator`, which it
// steps as a JS iterator: each part adapted, in one tuple or `Option`, so a
// write through the `&mut` reaches the impl's value and the iterator steps
// as it's bounded, from its back too (#2010). Built as a library, its
// dictionaries also give the layout, `ItemSize` and `ItemAlign`, of
// `Self::Item` that a consumer's generic code may ask; it prints the same.
pub trait Src {
    type Item;
    fn pair(&mut self) -> (&mut Self::Item, impl Iterator<Item = u32>);
    fn ends(&mut self) -> (impl DoubleEndedIterator<Item = u32>, &mut Self::Item);
    fn maybe(&mut self, open: bool) -> Option<(&mut Self::Item, impl Iterator<Item = u32>)>;
}

#[derive(Debug)]
struct Loud(Vec<u32>);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {:?}", self.0);
    }
}

struct Held(Loud);

impl Src for Held {
    type Item = Loud;
    fn pair(&mut self) -> (&mut Loud, impl Iterator<Item = u32>) {
        let n = self.0.0.len() as u32;
        (&mut self.0, (0..n).map(|x| x * 10))
    }
    fn ends(&mut self) -> (impl DoubleEndedIterator<Item = u32>, &mut Loud) {
        let items = self.0.0.clone();
        (items.into_iter(), &mut self.0)
    }
    fn maybe(&mut self, open: bool) -> Option<(&mut Loud, impl Iterator<Item = u32>)> {
        let n = self.0.0.len() as u32;
        open.then(move || (&mut self.0, 1..=n))
    }
}

struct Count(u32);

impl Src for Count {
    type Item = u32;
    fn pair(&mut self) -> (&mut u32, impl Iterator<Item = u32>) {
        let n = self.0;
        (&mut self.0, 0..n)
    }
    fn ends(&mut self) -> (impl DoubleEndedIterator<Item = u32>, &mut u32) {
        (0..self.0, &mut self.0)
    }
    fn maybe(&mut self, open: bool) -> Option<(&mut u32, impl Iterator<Item = u32>)> {
        let n = self.0;
        open.then(move || (&mut self.0, n..n + 2))
    }
}

struct Tagged<T>(T, u32);

impl<T: Clone> Src for Tagged<T> {
    type Item = T;
    fn pair(&mut self) -> (&mut T, impl Iterator<Item = u32>) {
        (&mut self.0, 0..self.1)
    }
    fn ends(&mut self) -> (impl DoubleEndedIterator<Item = u32>, &mut T) {
        (0..self.1, &mut self.0)
    }
    fn maybe(&mut self, open: bool) -> Option<(&mut T, impl Iterator<Item = u32>)> {
        let n = self.1;
        open.then(move || (&mut self.0, 0..n))
    }
}

fn read<T: Src>(t: &mut T, fresh: T::Item) -> u32 {
    let (held, it) = t.pair();
    *held = fresh;
    it.sum()
}

fn last<T: Src>(t: &mut T, fresh: T::Item) -> Option<u32> {
    let (mut it, held) = t.ends();
    *held = fresh;
    it.next_back()
}

fn opened<T: Src>(t: &mut T, open: bool, fresh: T::Item) -> Vec<u32> {
    match t.maybe(open) {
        Some((held, it)) => {
            *held = fresh;
            it.collect()
        }
        None => Vec::new(),
    }
}

fn main() {
    let mut held = Held(Loud(vec![1, 2, 3]));
    println!("pair {}", read(&mut held, Loud(vec![4, 5])));
    println!("held {:?}", held.0);
    println!("ends {:?}", last(&mut held, Loud(vec![6])));
    println!("held {:?}", held.0);
    println!("open {:?}", opened(&mut held, true, Loud(vec![7, 8])));
    println!("shut {:?}", opened(&mut held, false, Loud(vec![9])));
    println!("held {:?}", held.0);

    let mut count = Count(3);
    println!("pair {}", read(&mut count, 5));
    println!("count {}", count.0);
    println!("ends {:?}", last(&mut count, 2));
    println!("count {}", count.0);
    println!("open {:?}", opened(&mut count, true, 9));
    println!("shut {:?}", opened(&mut count, false, 1));
    println!("count {}", count.0);

    let mut tagged = Tagged((String::from("a"), 1u8), 3);
    println!("pair {}", read(&mut tagged, (String::from("b"), 2)));
    println!("ends {:?}", last(&mut tagged, (String::from("c"), 3)));
    println!("open {:?}", opened(&mut tagged, true, (String::from("d"), 4)));
    println!("tagged {:?}", tagged.0);
}
