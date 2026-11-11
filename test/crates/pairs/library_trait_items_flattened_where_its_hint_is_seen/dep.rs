pub trait Source {
    type Items: IntoIterator<Item = u32>;
    fn items(n: u32) -> Self::Items;
}

pub struct Ranges;

impl Source for Ranges {
    type Items = std::ops::Range<u32>;
    fn items(n: u32) -> std::ops::Range<u32> {
        0..n
    }
}
