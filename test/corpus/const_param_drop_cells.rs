// A struct of a const parameter with a destructor, as itertools' ArrayBuilder,
// in a crate that borrows a RefCell field: which cells are plain is asked of
// each body where it is, of its own parameters, not where the first asks.
use std::cell::RefCell;

struct Builder<T, const N: usize> {
    items: [Option<T>; N],
    len: usize,
}

impl<T, const N: usize> Builder<T, N> {
    fn new() -> Self {
        Builder { items: [(); N].map(|_| None), len: 0 }
    }

    fn push(&mut self, value: T) {
        self.items[self.len] = Some(value);
        self.len += 1;
    }
}

impl<T, const N: usize> Drop for Builder<T, N> {
    fn drop(&mut self) {
        println!("dropping {} of {}", self.len, N);
    }
}

fn filled<const N: usize>(word: &str) -> usize {
    let mut builder = Builder::<String, N>::new();
    for _ in 0..N {
        builder.push(word.to_string());
    }
    builder.len
}

struct Holder {
    cell: RefCell<Vec<u32>>,
}

fn main() {
    let h = Holder { cell: RefCell::new(Vec::new()) };
    h.cell.borrow_mut().push(1);
    println!("{:?} {}", h.cell.borrow(), filled::<3>("a"));
}
