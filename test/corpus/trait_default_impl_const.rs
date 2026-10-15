// A trait's default method, lowered in an impl's dictionary, calling a
// method of its `Self`, `Holder<T, CAP>`: given the impl's `CAP`, which
// `Self` names, as arrayvec's `ArrayVecImpl::as_slice` calls `self.len()`
// (ADR 0135); and another default, `full` calling `remaining`, of the
// impl's own dictionary.
trait Capacity {
    fn len(&self) -> usize;
    fn capacity(&self) -> usize;

    fn remaining(&self) -> usize {
        self.capacity() - self.len()
    }

    fn full(&self) -> bool {
        self.remaining() == 0
    }
}

struct Holder<T, const CAP: usize> {
    items: Vec<T>,
}

impl<T, const CAP: usize> Capacity for Holder<T, CAP> {
    fn len(&self) -> usize {
        self.items.len()
    }

    fn capacity(&self) -> usize {
        CAP
    }
}

fn main() {
    let h: Holder<u8, 3> = Holder { items: vec![1, 2, 3] };
    println!("{} {}", h.remaining(), h.full());
}
