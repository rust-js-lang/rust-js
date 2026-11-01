//@ run-fail: called `Option::unwrap()` on a `None` value
// A `Cell<Option<T>>` set to a `Some`, then read back by the next
// statement before any other code runs, ReactART's Surface's
// `self.surface.set(Some(..)); create(self.surface.get().unwrap(), ..)`:
// its `unwrap` is the value, `this.surface = ..; create(this.surface, ..)`,
// as nothing could have taken it out (rd331e). Code between, a call that
// runs first or a statement apart, keeps Rust's check, which then panics.
use std::cell::Cell;

// `pub`, so it stays an object whose fields are cells, no function's scope
// (ADR 1043).
pub struct Surface {
    canvas: Cell<Option<&'static str>>,
    size: Cell<Option<f64>>,
}

fn shown(canvas: &str, size: f64) -> String {
    format!("{canvas} at {size}")
}

fn joined(size: f64, canvas: &str) -> String {
    format!("{size}: {canvas}")
}

fn noted(why: &'static str) -> &'static str {
    println!("noted: {why}");
    why
}

impl Surface {
    fn mount(&self, name: &'static str) {
        self.canvas.set(Some(name));
        let line = shown(self.canvas.get().unwrap(), 2.0);
        println!("{line}");
        self.size.set(Some(3.0));
        let size = self.size.get().expect("set just now");
        println!("{size}");
        // A message that runs code runs it still.
        self.size.set(Some(size + 1.0));
        let size = self.size.get().expect(noted("set again"));
        println!("{size}");
        // Through a block that only binds, `_ = ..`'s.
        self.canvas.set(Some("framed"));
        _ = self.size.replace(Some(shown(self.canvas.get().unwrap(), size).len() as f64));
        println!("{:?}", self.size.get());
    }

    // Read back as what the body gives.
    fn framed(&self) -> String {
        self.canvas.set(Some("tail"));
        shown(self.canvas.get().unwrap(), 0.5)
    }

    fn unset(&self) -> f64 {
        self.canvas.set(None);
        1.0
    }

    fn remount(&self) {
        // Another cell's read back is no read of this one.
        self.size.set(Some(4.0));
        let other = self.canvas.get().unwrap();
        println!("{other}");
        // A call that runs first may unset it, here as an argument before.
        self.canvas.set(Some("again"));
        let line = joined(self.unset(), self.canvas.get().unwrap());
        println!("{line}");
    }
}

fn main() {
    let surface = Surface {
        canvas: Cell::new(None),
        size: Cell::new(None),
    };
    surface.mount("canvas");
    println!("{}", surface.framed());
    surface.remount();
}
