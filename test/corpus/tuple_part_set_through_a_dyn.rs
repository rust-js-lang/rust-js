// A tuple matched, `match (v.pop(), x.0)`, its `x.0` read where the tuple's
// made, before a guard's call sets it through a `&mut dyn` it's given,
// `poke(if first { &mut x.0 } else { &mut y.0 })`: what the metamorphic
// net's `dyn-ref` rewrite found of `field_rewrite_singles` (#1035), the
// pair made on another name of `x`, or of a part of it, `x.0` of `x.0.0`,
// which the read was moved past.
struct Held<T>(T);

trait Bump {
    fn bump(&mut self) -> bool;
}

impl Bump for u32 {
    fn bump(&mut self) -> bool {
        *self += 10;
        false
    }
}

fn poke(n: &mut dyn Bump) -> bool {
    n.bump()
}

fn bumped_through_dyn(v: &mut Vec<u32>, first: bool) -> u32 {
    let mut x = Held(1);
    let mut y = Held(2);
    match (v.pop(), x.0) {
        (_, _) if poke(if first { &mut x.0 } else { &mut y.0 }) => 0,
        (Some(b), a) => a * 100 + b + x.0 + y.0,
        (None, a) => a + x.0 + y.0,
    }
}

// And a part of a part, `x.0.0`, the pair made on another name of `x.0`.
fn bumped_through_a_part(v: &mut Vec<u32>, first: bool) -> u32 {
    let mut x = Held(Held(1));
    let mut y = Held(Held(2));
    match (v.pop(), x.0.0) {
        (_, _) if poke(if first { &mut x.0.0 } else { &mut y.0.0 }) => 0,
        (Some(b), a) => a * 100 + b + x.0.0 + y.0.0,
        (None, a) => a + x.0.0 + y.0.0,
    }
}

fn main() {
    println!(
        "{} {}",
        bumped_through_a_part(&mut vec![6], true),
        bumped_through_a_part(&mut vec![], false)
    );
    println!(
        "{} {}",
        bumped_through_dyn(&mut vec![4], true),
        bumped_through_dyn(&mut vec![], false)
    );
    println!(
        "{} {}",
        bumped_through_dyn(&mut vec![], true),
        bumped_through_dyn(&mut vec![5], false)
    );
}
