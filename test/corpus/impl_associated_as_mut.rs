//@ library-differs
// An impl's `as_mut()` of its associated type, an array it knows, and a
// generic caller's through its dictionary.

trait Collect {
    type Buffer: AsMut<[Option<u8>]> + Default + std::fmt::Debug;
    fn fill(buf: &mut Self::Buffer);
}

impl Collect for (u8, u8, u8) {
    type Buffer = [Option<u8>; 2];

    fn fill(buf: &mut Self::Buffer) {
        let s = buf.as_mut();
        if !s.is_empty() {
            s[0] = Some(1);
        }
    }
}

fn run<T: Collect>() -> T::Buffer {
    let mut b = T::Buffer::default();
    T::fill(&mut b);
    b
}

fn main() {
    println!("{:?}", run::<(u8, u8, u8)>());
    let mut direct = [None, None];
    <(u8, u8, u8)>::fill(&mut direct);
    println!("{direct:?}");
}
