// A bound of any lifetime, `for<'a> &'a Self: IntoIterator`, read where a
// call is checked for a collection of the crate's: its lifetime erased, not
// left unbound, which rustc can't select an impl for. From rustc's
// `higher-ranked/trait-bounds/assoc-type-projection-method.rs`.

trait Collection
where
    for<'a> &'a Self: IntoIterator,
{
    fn my_iter(&self) -> <&Self as IntoIterator>::IntoIter {
        self.into_iter()
    }
}

impl<T> Collection for [T] {}

fn main() {
    let v = [3usize, 4];
    let total: usize = v.my_iter().sum();
    println!("{} {}", total, v[..].my_iter().count());
}
