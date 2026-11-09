//@ library-differs
// `size_of` and `align_of` of `<I as Iterator>::Item`, a std associated
// type of a type parameter: one of its slots, its caller's to give, as a
// map's keys' class is (ADR 0466). And of a trait's default's `<Self as
// Iterator>::Item`: each impl's that copies it, as a collect's is, its own
// type for it, a blanket impl's slot or a known one's (#990).
use std::mem::{align_of, size_of};

fn item_size<I: Iterator>(_items: &I) -> usize {
    size_of::<I::Item>()
}

fn item_align<I: IntoIterator>(_items: &I) -> usize {
    align_of::<I::Item>()
}

fn relayed<I: Iterator>(items: &I) -> (usize, usize) {
    (item_size(items), item_align(items))
}

trait Layout: Iterator {
    fn item_layout(&self) -> (usize, usize) {
        (size_of::<Self::Item>(), align_of::<Self::Item>())
    }

    fn collect_layout(self) -> (Vec<Self::Item>, usize)
    where
        Self: Sized,
    {
        (self.collect(), size_of::<Self::Item>())
    }
}

impl<I: Iterator> Layout for std::iter::Peekable<I> {}

struct Pairs(u8);

impl Iterator for Pairs {
    type Item = (u32, u8);

    fn next(&mut self) -> Option<(u32, u8)> {
        if self.0 == 0 {
            return None;
        }
        self.0 -= 1;
        Some((u32::from(self.0) * 70_000, self.0))
    }
}

impl Layout for Pairs {}

fn layout_of<T: Layout>(items: &T) -> (usize, usize) {
    items.item_layout()
}

fn main() {
    println!("{}", item_size(&(0..3u16)));
    println!("{}", item_size(&[1u64].into_iter()));
    println!("{}", item_size(&"ab".chars()));
    println!("{}", item_align(&vec![1u8, 2]));
    println!("{:?}", relayed(&(0..1u32)));
    println!("{:?}", (0..2u8).peekable().item_layout());
    println!("{:?}", Pairs(1).item_layout());
    println!("{:?}", layout_of(&[1u64, 2].into_iter().peekable()));
    println!("{:?}", Pairs(2).collect_layout());
    println!("{:?}", "xy".chars().peekable().collect_layout());
}
