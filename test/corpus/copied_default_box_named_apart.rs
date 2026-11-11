// A copied default's `&mut self` that generic code gives in a box, of a
// type with no name of its own, an array's, is named `boxed` before its
// body is, so the body's own `boxed` is another name, not the parameter's
// twice, which JS refuses (#803, #1109).
trait Touch {
    fn touch(&mut self, by: &mut u32);

    fn touch_twice(&mut self, by: &mut u32) {
        let boxed = *by + 1;
        self.touch(by);
        *by += boxed;
        self.touch(by);
        *by += boxed;
    }
}

struct Holder<T> {
    t: T,
    n: u32,
}

impl<T: Copy> Touch for Holder<T> {
    fn touch(&mut self, by: &mut u32) {
        self.n += *by;
        *by += 1;
    }
}

impl Touch for [u32; 2] {
    fn touch(&mut self, by: &mut u32) {
        self[0] += *by;
    }
}

impl Touch for u32 {
    fn touch(&mut self, by: &mut u32) {
        *self += *by;
    }
}

fn twice<A: Touch>(a: &mut A, by: &mut u32) {
    a.touch_twice(by);
}

fn main() {
    let mut h = Holder { t: 1u8, n: 0 };
    let mut by = 2;
    twice(&mut h, &mut by);
    println!("{} {} {}", h.t, h.n, by);
    let mut n = 5u32;
    twice(&mut n, &mut by);
    println!("{n} {by}");
    let mut pair = [1u32, 2];
    twice(&mut pair, &mut by);
    println!("{pair:?} {by}");
}
