// A generic impl's dictionary, `holderTouch(TTouch)`, that a body asks
// twice or more is read once at its top (ADR 0979, amended, #691): in a
// copied default of the impl, in generic code, in a closure, behind a
// branch not taken, and of a type known whole.

trait Touch {
    fn touch(&mut self);
    fn count(&self) -> u32;

    fn twice(&mut self) {
        self.touch();
        self.touch();
    }

    fn four(&mut self) {
        self.twice();
        self.twice();
    }

    fn maybe_four(&mut self, go: bool) -> u32 {
        if go {
            self.twice();
            self.twice();
        }
        self.count()
    }
}

struct Holder<T> {
    t: T,
    touched: u32,
}

impl<T: Touch> Touch for Holder<T> {
    fn touch(&mut self) {
        self.t.touch();
        self.touched += 1;
    }

    fn count(&self) -> u32 {
        self.t.count() * 100 + self.touched
    }
}

impl Touch for u32 {
    fn touch(&mut self) {
        *self += 1;
    }

    fn count(&self) -> u32 {
        *self
    }
}

fn both<T: Touch>(h: &mut Holder<T>) -> u32 {
    h.twice();
    h.twice();
    h.count()
}

fn in_a_closure<T: Touch>(h: &mut Holder<T>, times: u32) -> u32 {
    let mut run = |n: u32| {
        for _ in 0..n {
            h.touch();
        }
        h.twice();
        h.twice();
    };
    run(times);
    run(1);
    h.count()
}

fn only_if<T: Touch>(h: &mut Holder<T>, go: bool) -> u32 {
    if go {
        h.twice();
        h.four();
    }
    h.count()
}

// An impl whose dictionary is given a drop, `slotRenew(TDefault, dropT)`,
// asked twice in generic code given that drop.
trait Renew {
    fn renew(&mut self);

    fn renew_twice(&mut self) {
        self.renew();
        self.renew();
    }
}

struct Slot<T> {
    value: T,
    renewed: u32,
}

impl<T: Default> Renew for Slot<T> {
    fn renew(&mut self) {
        self.value = T::default();
        self.renewed += 1;
    }
}

#[derive(Default)]
struct Loud(u32);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn renewed<T: Default>(slot: &mut Slot<T>) -> u32 {
    slot.renew_twice();
    slot.renew_twice();
    slot.renewed
}

fn main() {
    let mut slot = Slot {
        value: Loud(7),
        renewed: 0,
    };
    println!("{}", renewed(&mut slot));
    let mut h = Holder { t: 1u32, touched: 0 };
    println!("{}", both(&mut h));
    h.four();
    h.four();
    println!("{}", h.count());
    println!("{}", h.maybe_four(false));
    println!("{}", h.maybe_four(true));
    println!("{}", in_a_closure(&mut h, 2));
    println!("{}", only_if(&mut h, false));
    println!("{}", only_if(&mut h, true));
    let mut nested = Holder {
        t: Holder { t: 5u32, touched: 0 },
        touched: 0,
    };
    println!("{}", both(&mut nested));
    nested.four();
    println!("{}", nested.maybe_four(true));
    println!("{}", only_if(&mut nested, false));
}
