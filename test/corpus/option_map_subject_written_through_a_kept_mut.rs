// `slot.x.map(|v| reset(&mut slot.x) + v)`: `map` takes a copy of the
// `Option`, so its closure may write the field through a `&mut` of it,
// here one `reset` hands on to generic code, kept, writing the field
// through. The value `map` gives the closure is the copy's, read before
// `reset` runs: `Some(101)`, not the field read again after it (#2543;
// cut by `scripts/reduce.ts` from `option_subject_of_a_cells_field`'s
// `observe-borrows` rewrite).
struct Slot {
    x: Option<i32>,
}
fn reset(x: &mut Option<i32>) -> i32 {
    *x = None;
    observe::at_31(&x);
    100
}
fn reset_after() -> (Option<i32>, Option<i32>) {
    let mut slot = Slot { x: Some(1) };
    let r = slot.x.map(|v| reset(&mut slot.x) + v);
    (r, slot.x)
}
fn main() {
    println!("{:?}", reset_after());
}
mod observe {
    use ::std::rc::Rc;
    use ::std::sync::Arc;
    pub struct Seen {
        lines: Vec<String>,
        depth: usize,
        guarded: bool,
    }
    impl Seen {
        pub fn walk<T: ?Sized + Observe>(&mut self, x: &T, path: &str, guarded: bool) {
            self.guarded |= guarded;
            x.observe(path, self);
        }
    }
    pub trait Observe {
        fn observe(&self, path: &str, seen: &mut Seen);
    }
    fn point<T: ?Sized + Observe>(k: usize, name: &str, x: &T) {
        let mut seen = Seen {
            lines: Vec::new(),
            depth: 0,
            guarded: false,
        };
        seen.walk(x, name, false);
        let _ = (k, seen.lines.len(), seen.depth, seen.guarded);
    }
    impl<T: Observe> Observe for Option<T> {
        fn observe(&self, _: &str, _: &mut Seen) {}
    }
    impl<T: ?Sized + Observe> Observe for Box<T> {
        fn observe(&self, path: &str, seen: &mut Seen) {
            seen.walk(&**self, path, false);
        }
    }
    impl<T: ?Sized + Observe> Observe for Rc<T> {
        fn observe(&self, path: &str, seen: &mut Seen) {
            seen.walk(&**self, path, false);
        }
    }
    impl<T: ?Sized + Observe> Observe for Arc<T> {
        fn observe(&self, path: &str, seen: &mut Seen) {
            seen.walk(&**self, path, false);
        }
    }
    impl<'a, T: ?Sized + Observe> Observe for &'a mut T {
        fn observe(&self, _: &str, _: &mut Seen) {}
    }
    impl Observe for i32 {
        fn observe(&self, _: &str, _: &mut Seen) {}
    }
    pub fn at_31<T: ?Sized + Observe>(x: &T) {
        point(0usize, "", x)
    }
}
