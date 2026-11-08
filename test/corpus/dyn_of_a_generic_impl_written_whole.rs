//@ library-differs
// A `&mut` to an object made a `&mut dyn Trait` whose impl is generic,
// `impl<T> Trait for T`, which has its `&mut self` as generic code's box
// (ADR 0099): a write of the whole, `*self = v` or through the `&mut T` it
// gives back, changes the object in place, as an in-place cell's does (ADR
// 0454, #968). A pair holds the place it was made of: a reborrow's of what
// the reference was then, an index as it was then.
#[derive(Debug, Default)]
struct C {
    n: i32,
}

trait Reset {
    fn reset(&mut self);
}

impl<T: Default> Reset for T {
    fn reset(&mut self) {
        *self = T::default();
    }
}

trait GetMut<T> {
    fn get_mut(&mut self) -> &mut T;
}

impl<T> GetMut<T> for T {
    fn get_mut(&mut self) -> &mut T {
        self
    }
}

fn reset(r: &mut dyn Reset) {
    r.reset();
}

fn set<T>(x: &mut dyn GetMut<T>, v: T) {
    *x.get_mut() = v;
}

fn swapped<T>(a: &mut dyn GetMut<T>, b: &mut dyn GetMut<T>) {
    std::mem::swap(a.get_mut(), b.get_mut());
}

fn add(a: &mut dyn GetMut<i32>, b: i32) {
    *a.get_mut() += b;
}

fn add_at(lanes: &mut [i32; 4], mut index: usize) -> usize {
    add(&mut lanes[index], {
        index += 1;
        20
    });
    index
}

fn main() {
    let mut c = C { n: 5 };
    reset(&mut c);
    let mut d = C { n: 5 };
    set(&mut d, C { n: 20 });
    println!("{c:?} {d:?}");
    swapped(&mut c, &mut d);
    println!("{c:?} {d:?}");
    let mut v = vec![1, 2];
    let mut w = vec![3];
    set(&mut v, vec![4, 5, 6]);
    swapped(&mut v, &mut w);
    println!("{v:?} {w:?}");
    let mut e = C { n: 1 };
    {
        let r: &mut dyn Reset = &mut e;
        r.reset();
    }
    println!("{e:?}");

    let mut p = 1;
    let mut q = 2;
    let mut x = &mut p;
    add(&mut *x, {
        x = &mut q;
        10
    });
    *x += 100;
    println!("{p} {q}");
    let mut lanes = [0; 4];
    let next = add_at(&mut lanes, 2);
    println!("{lanes:?} {next}");
}
