//@ library-differs
// A trait method that gives a `&mut` to its associated type, called through
// a dictionary: generic code has it in a box, whatever the impl gives.

trait Get {
    type Out;
    fn get(&mut self) -> &mut Self::Out;
}

struct Holder(Vec<u32>);

impl Get for Holder {
    type Out = Vec<u32>;
    fn get(&mut self) -> &mut Vec<u32> {
        &mut self.0
    }
}

struct Num(u32);

impl Get for Num {
    type Out = u32;
    fn get(&mut self) -> &mut u32 {
        &mut self.0
    }
}

struct Slot<T>(T);

impl<T> Get for Slot<T> {
    type Out = T;
    fn get(&mut self) -> &mut T {
        &mut self.0
    }
}

fn push<G: Get<Out = Vec<u32>>>(g: &mut G) {
    g.get().push(1);
}

fn first<G: Get>(g: &mut G) -> &mut G::Out {
    g.get()
}

fn reset<G: Get>(g: &mut G, to: G::Out) {
    *g.get() = to;
}

fn bump<G: Get<Out = u32>>(g: &mut G) {
    *g.get() += 1;
}

fn main() {
    let mut h = Holder(vec![5]);
    push(&mut h);
    first(&mut h).push(2);
    println!("{:?}", h.0);
    let kept = &h.0 as *const Vec<u32>;
    reset(&mut h, vec![7, 8]);
    *first(&mut h) = vec![9];
    println!("{:?} {}", h.0, std::ptr::eq(kept, &h.0));

    let mut n = Num(1);
    bump(&mut n);
    *first(&mut n) += 5;
    reset(&mut n, 40);
    bump(&mut n);
    println!("{}", n.0);

    let mut slot = Slot(vec![1]);
    push(&mut slot);
    first(&mut slot).push(2);
    reset(&mut slot, vec![3, 4]);
    first(&mut slot).push(5);
    let mut count = Slot(1u32);
    bump(&mut count);
    println!("{:?} {}", slot.0, count.0);
}
