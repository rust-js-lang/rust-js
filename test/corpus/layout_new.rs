// `Layout::new::<T>()`, of a type and of a type parameter given its size
// and alignment (ADRs 0145, 0417), and `Layout::from_size_align_unchecked`.
use std::alloc::Layout;

fn layout_of<T>() -> Layout {
    Layout::new::<T>()
}

fn main() {
    println!("{:?}", Layout::new::<u64>());
    println!("{:?} {:?}", layout_of::<(u8, u32)>(), layout_of::<[u16; 3]>());
    let l = unsafe { Layout::from_size_align_unchecked(24, 8) };
    println!("{} {} {:?}", l.size(), l.align(), l);
}
