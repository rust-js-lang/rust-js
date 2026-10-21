// `as_mut()` and `as_ref()` of an associated type, `T::Buffer`, bound by
// `AsMut` and `AsRef`: its dictionary's, as a type parameter's is, here an
// array's own items, changed in place (ADR 0533).
trait Collect {
    type Item;
    type Buffer: Default + AsRef<[Option<Self::Item>]> + AsMut<[Option<Self::Item>]>;
}

impl<A> Collect for (A, A) {
    type Item = A;
    type Buffer = [Option<A>; 2];
}

struct Buffer<T: Collect> {
    cur: usize,
    buf: T::Buffer,
}

impl<T: Collect> Buffer<T> {
    fn new(items: impl IntoIterator<Item = T::Item>) -> Self {
        let mut buf = T::Buffer::default();
        for (slot, item) in buf.as_mut().iter_mut().zip(items) {
            *slot = Some(item);
        }
        Buffer { cur: 0, buf }
    }

    fn held(&self) -> usize {
        self.buf.as_ref().iter().filter(|item| item.is_some()).count()
    }
}

impl<T: Collect> Iterator for Buffer<T> {
    type Item = T::Item;

    fn next(&mut self) -> Option<T::Item> {
        let s = self.buf.as_mut();
        if let Some(item) = s.get_mut(self.cur) {
            self.cur += 1;
            item.take()
        } else {
            None
        }
    }
}

fn main() {
    let mut words = Buffer::<(String, String)>::new(vec!["a".to_string()]);
    println!("{}", words.held());
    println!("{:?} {}", words.next(), words.held());
    println!("{:?} {:?}", words.next(), words.next());
    let numbers: Vec<u32> = Buffer::<(u32, u32)>::new([4, 5, 6]).collect();
    println!("{numbers:?}");
}
