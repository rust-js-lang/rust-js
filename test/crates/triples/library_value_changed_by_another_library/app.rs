//! Gives `mid` what `dep` hands out, which `mid` changes in place though
//! neither `dep` nor this crate does: `dep`'s constant is as it was after
//! each (#1907).
use mid::Mover;
use std::cell::RefCell;

fn moved_by<M: Mover>(mover: &M, spot: &mut dep::Spot) {
    mover.moved(spot);
}

pub fn main() {
    println!("{:?} {:?}", mid::nudged(dep::origin()), dep::origin());
    let mut origin = dep::origin();
    mid::shift(&mut origin);
    println!("{origin:?} {:?}", dep::origin());
    let cells = mid::Cells {
        spot: RefCell::new(dep::origin()),
    };
    mid::bump(&cells);
    let bumped = *cells.spot.borrow();
    println!("{bumped:?} {:?}", dep::origin());
    let mover: &dyn Mover = &mid::Up;
    let mut origin = dep::origin();
    mover.moved(&mut origin);
    println!("{origin:?} {:?}", dep::origin());
    let mut origin = dep::origin();
    moved_by(&mid::Up, &mut origin);
    println!("{origin:?} {:?}", dep::origin());
    println!("{:?} {:?}", mid::with_spot(dep::origin), dep::origin());
    let mut origin = dep::origin();
    mid::aim(mid::Target { spot: &mut origin });
    println!("{origin:?} {:?}", dep::origin());
    // What a cell `dep` hands out an `Rc` of holds, read and changed.
    let store = dep::store();
    let mut read = dep::read(&store);
    mid::shift(&mut read);
    println!("{read:?} {:?}", dep::read(&store));
    println!("{}", dep::origin_x());
    // A thread-local's, and what a stored value holds.
    let mut home = dep::home();
    mid::marked(&mut home);
    println!("{home:?} {:?}", dep::home());
    let mut pair = dep::pair();
    mid::shift_first(&mut pair);
    println!("{pair:?} {:?}", dep::pair());
    // What an `Rc` the consumer holds points at.
    let shared = dep::shared();
    let mut got = dep::got(&shared);
    mid::marked(&mut got);
    println!("{got:?} {:?} {}", dep::got(&shared), dep::got_n(&shared));
}
