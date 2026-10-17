// `Default` of an array of a caller's `N`, which Rust asks a where clause
// for, `[T; N]: Default`: that bound's dictionary's, which the caller makes
// of its own `N` (ADR 0107; rustc's const-generics/type-dependent/issue-69816.rs).
trait IterExt: Sized + Iterator {
    fn default_for_size<const N: usize>(self) -> [Self::Item; N]
    where
        [Self::Item; N]: Default,
    {
        Default::default()
    }
}

impl<T: Iterator> IterExt for T {}

fn zeros<T: Default, const N: usize>() -> [T; N]
where
    [T; N]: Default,
{
    Default::default()
}

fn defaults<const N: usize>() -> [String; N]
where
    [String; N]: Default,
{
    Default::default()
}

fn main() {
    let arr = (0u32..10).default_for_size::<10>();
    println!("{arr:?}");
    let z: [u8; 3] = zeros();
    println!("{z:?}");
    let mut words = defaults::<3>();
    words[0].push('a');
    println!("{words:?}");
}
