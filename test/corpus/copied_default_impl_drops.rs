// A copied default that calls its impl's own functions, or another default
// through its impl's dictionary, of an impl for a generic type of the
// crate's, `impl<I: Iterator> Trait for Holder<I>`, gives them the impl's
// drops, `dropI`, not its trait's `Self`'s, whose index the impl's
// `I` shares (ADR 0495).
struct Loud(u8);
impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Default)]
struct Gen(u8, Tag);

#[derive(Default)]
struct Tag(u8);

impl Drop for Tag {
    fn drop(&mut self) {
        println!("tag {}", self.0);
    }
}
impl Iterator for Gen {
    type Item = Loud;
    fn next(&mut self) -> Option<Loud> {
        self.0 += 1;
        if self.0 < 5 { Some(Loud(self.0)) } else { None }
    }
}

trait Skipping: Iterator {
    fn skip_one(&mut self) {
        self.next();
    }

    fn eat(self)
    where
        Self: Sized,
    {
        println!("eaten");
    }

    fn eat_after_one(mut self)
    where
        Self: Sized,
    {
        self.skip_one();
        self.eat();
    }

    fn skip_two(&mut self) {
        self.skip_one();
        self.skip_one();
    }
}

impl<I: Iterator + Default> Iterator for Holder<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        let next = self.0.next();
        if next.is_none() {
            self.0 = I::default();
        }
        next
    }
}

impl<I: Iterator + Default> Skipping for Holder<I> {}

trait Skip {
    fn skip(&mut self);
}

struct Holder<I>(I);

impl<I: Iterator> Skip for Holder<I> {
    fn skip(&mut self) {
        self.0.next();
    }
}

fn main() {
    let mut g = Holder(Gen(0, Tag(1)));
    g.skip_two();
    println!("two skipped");
    let mut h = Holder(Gen(2, Tag(2)));
    let d: &mut dyn Skip = &mut h;
    d.skip();
    println!("dyn skipped");
    Holder(Gen(0, Tag(7))).eat_after_one();
    let mut spent = Holder(Gen(4, Tag(8)));
    spent.skip_one();
    println!("spent");
    println!("end");
}
