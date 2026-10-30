//@ library-differs
// A trait impl's own `T: Copy` bound takes its copy function with its
// dictionary, as rustc's traits/conditional-dispatch.rs dispatches through
// one: a copy of an `Option<Point>`, or of a `[Point; 2]`, made by the
// impl's method is its own, so changing it leaves what it copied.
#[derive(Clone, Copy, Debug)]
struct Point {
    x: u32,
}

trait Again {
    fn again(&self) -> Self;
}

impl<T: Copy> Again for Option<T> {
    fn again(&self) -> Self {
        *self
    }
}

impl<T: Copy> Again for [T; 2] {
    fn again(&self) -> Self {
        *self
    }
}

trait Get {
    fn get(&self) -> Self;
}

impl<T: Again> Get for T {
    fn get(&self) -> T {
        self.again()
    }
}

fn get_it<T: Get>(t: &T) -> T {
    t.get()
}

fn main() {
    let p = Some(Point { x: 4 });
    let mut q = get_it(&p);
    if let Some(q) = &mut q {
        q.x += 1;
    }
    println!("{p:?} {q:?}");
    let pair = [Point { x: 1 }, Point { x: 2 }];
    let mut copy = get_it(&pair);
    copy[0].x = 9;
    println!("{pair:?} {copy:?}");
}
