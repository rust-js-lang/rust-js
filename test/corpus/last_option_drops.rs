// `last()` of an iterator of `Option`s with a destructor: `Some` of the last
// item, which is `Some(None)` where that item is `None` (ADR 0051), each
// before it dropped as the next comes (ADR 0098, #204).
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn main() {
    let none_last = [Some(Loud(1)), None].into_iter().last();
    println!("{} {}", none_last.is_some(), none_last.as_ref().is_some_and(|l| l.is_none()));

    let some_last = vec![None, Some(Loud(2))].into_iter().last();
    println!("{:?}", some_last.as_ref().map(|l| l.as_ref().map(|l| l.0)));

    let chained = [Some(Loud(3))].into_iter().chain(vec![None]).last();
    println!("{}", chained.is_some());

    let empty: Option<Option<Loud>> = Vec::<Option<Loud>>::new().into_iter().last();
    println!("{}", empty.is_none());
}
