// An `Rc` written through a `&mut` (ADR 0147, amended): the place gets
// another, and what the old one pointed at, which its clones share, stays
// as it was; so a clone of one is the same object, as React's update queue
// clones `pending.next.borrow().clone()` and assigns it.
use std::cell::RefCell;
use std::rc::Rc;

struct Update {
    lane: u32,
    next: RefCell<Rc<Lane>>,
}

struct Lane {
    bits: u32,
}

struct Holder {
    rc: Rc<Lane>,
}

fn set(r: &mut Rc<Lane>, v: Rc<Lane>) {
    *r = v;
}

fn link(update: &Update, pending: &Update) {
    let next = pending.next.borrow().clone();
    *update.next.borrow_mut() = next;
}

fn main() {
    let first = Rc::new(Lane { bits: 1 });
    let mut holder = Holder { rc: Rc::clone(&first) };
    set(&mut holder.rc, Rc::new(Lane { bits: 2 }));
    println!("{} {}", first.bits, holder.rc.bits);

    let update = Update { lane: 1, next: RefCell::new(Rc::clone(&first)) };
    let pending = Update { lane: 2, next: RefCell::new(Rc::new(Lane { bits: 3 })) };
    link(&update, &pending);
    println!(
        "{} {} {} {}",
        first.bits,
        update.next.borrow().bits,
        Rc::ptr_eq(&update.next.borrow(), &pending.next.borrow()),
        update.lane + pending.lane
    );

    let mut a = Rc::clone(&first);
    let mut b = Rc::new(Lane { bits: 4 });
    std::mem::swap(&mut a, &mut b);
    let old = std::mem::replace(&mut a, Rc::clone(&first));
    println!("{} {} {} {}", a.bits, b.bits, old.bits, first.bits);
}
