// A copied default's `Vec<Self::Item>` of an impl that isn't a blanket one,
// `<Wrapper<I> as Iterator>::Item` in its impl's terms, is asked what it
// drops normalized, `I::Item`'s, as a blanket impl's `<T as Iterator>::Item`
// is: a `match` of a pair of them that moves a part in one arm and the whole
// in another drops each part once, where its arm or its owner leaves it, as
// MIR's lowering does (#1337). It compiled once where the alias, asked as
// it was, was taken for one that drops nothing, correct only by luck.
struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Wrapper<I>(I);

impl<I: Iterator> Iterator for Wrapper<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.0.next()
    }
}

trait Ext: Iterator {
    fn halves(mut self, whole: bool) -> usize
    where
        Self: Sized,
    {
        let front: Vec<Self::Item> = self.by_ref().take(2).collect();
        let back: Vec<Self::Item> = self.collect();
        let pair = (front, back);
        match pair {
            (front, _) if !whole => {
                println!("front only");
                front.len()
            }
            both => {
                println!("both");
                both.0.len() + both.1.len()
            }
        }
    }
}

impl<I: Iterator> Ext for Wrapper<I> {}

fn main() {
    println!("{}", Wrapper((1..5).map(Noisy)).halves(false));
    println!("{}", Wrapper((5..8).map(Noisy)).halves(true));
}
