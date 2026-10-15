// A `&mut` of a local given to a function that may keep it, where another
// parameter's type, or the result's, names its lifetime: a handle on the
// local, which writes through it after the call reach, not a box copied
// back when the call returns (ADR 0099).
struct Holder<'a> {
    held: Option<&'a mut u32>,
}

impl<'a> Holder<'a> {
    fn hold(&mut self, x: &'a mut u32) {
        self.held = Some(x);
    }

    fn bump(&mut self) {
        if let Some(x) = self.held.take() {
            *x += 1;
            self.held = Some(x);
        }
    }
}

fn keep<'a>(kept: &mut Vec<&'a mut u32>, x: &'a mut u32) {
    kept.push(x);
}

/// One whose lifetime names no other parameter's: a box, copied back.
fn doubled(x: &mut u32) {
    *x *= 2;
}

fn main() {
    let mut a = 10;
    let mut kept = Vec::new();
    keep(&mut kept, &mut a);
    *kept[0] += 1;
    drop(kept);
    let mut b = 20;
    let mut holder = Holder { held: None };
    holder.hold(&mut b);
    holder.bump();
    holder.bump();
    drop(holder);
    let mut c = 3;
    doubled(&mut c);
    println!("{a} {b} {c}");
}
