// A blanket impl's default copied into its impl, whose body makes a value
// of its `Self::Item`s with a destructor, `Vec::from_iter(self)` of
// itertools' `sorted_unstable`: what its THIR, in its trait's terms, makes
// and moves is asked what it drops in the impl's, `<T as Iterator>::Item`
// a caller gives the drop of, where rust-js refused `<Self as
// Iterator>::Item` (#828). What's sorted drops in order, the rest where the
// sorted iterator does.
struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

trait Ext: Iterator {
    fn sorted_by_key_unstable<K: Ord>(self, key: impl Fn(&Self::Item) -> K) -> std::vec::IntoIter<Self::Item>
    where
        Self: Sized,
    {
        let mut v = Vec::from_iter(self);
        v.sort_unstable_by_key(|item| key(item));
        v.into_iter()
    }

    fn sorted_unstable(self) -> std::vec::IntoIter<Self::Item>
    where
        Self: Sized,
        Self::Item: Ord,
    {
        let mut v = Vec::from_iter(self);
        v.sort_unstable();
        v.into_iter()
    }
}

impl<T: Iterator + ?Sized> Ext for T {}

// And of an impl that isn't a blanket one, whose `Self::Item` is
// `<Wrapper<I> as Iterator>::Item` in its terms, `I::Item` where it's
// normalized.
struct Wrapper<I>(I);

impl<I: Iterator> Iterator for Wrapper<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

trait Reversed: Iterator {
    fn reversed(self) -> Vec<Self::Item>
    where
        Self: Sized,
    {
        let mut v: Vec<Self::Item> = self.collect();
        v.reverse();
        v
    }
}

impl<I: Iterator> Reversed for Wrapper<I> {}

fn main() {
    let mut sorted = [3, 1, 2].into_iter().map(Noisy).sorted_by_key_unstable(|n| n.0);
    if let Some(first) = sorted.next() {
        println!("first {}", first.0);
    }
    drop(sorted);
    let words: Vec<String> = ["pear", "fig", "apple"].map(String::from).into_iter().sorted_unstable().collect();
    println!("{words:?}");
    let reversed = Wrapper([4, 5, 6].into_iter().map(Noisy)).reversed();
    println!("reversed {}", reversed.len());
}
