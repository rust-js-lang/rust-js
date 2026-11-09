//@ native: wasm32
// An `Option`'s payload is inside it, past its tag where it has no niche,
// where rustc's layout puts it: lent by `as_ref`, `as_mut`, `iter`,
// `get_or_insert` and `insert`, or matched, `if let Some(p) = &o`, its
// address is the `Option`'s and that offset, in a local, a tuple or a
// struct's field. JS holds an `Option` of an object as the object, so a
// pointer to the `Option` itself made of a reference to it is one to what
// holds the object, the object's address less the offset, as one to a
// `RefCell` of an object, which is it, is (#1114). Each was the object's own
// allocation, or the `Option`'s place: `4096` or `0` where wasm32 gives `4`.
use std::cell::RefCell;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
struct P {
    a: u8,
    b: u32,
}

struct Slot {
    tag: u8,
    held: Option<P>,
}

fn outer(o: &Option<P>) -> usize {
    o as *const Option<P> as usize
}

fn of_cell(cell: &RefCell<P>) -> usize {
    cell as *const RefCell<P> as usize
}

fn main() {
    let mut o = Some(P { a: 1, b: 2 });
    let base = &raw const o as usize;
    println!("as_ref {}", o.as_ref().unwrap() as *const P as usize - base);
    if let Some(p) = &o {
        println!("matched {} {}", p as *const P as usize - base, &raw const p.b as usize - base);
    }
    println!("as_mut {}", o.as_mut().unwrap() as *mut P as usize - base);
    println!("iter {}", o.iter().next().unwrap() as *const P as usize - base);
    println!("by reference {}", o.as_ref().unwrap() as *const P as usize - outer(&o));

    let mut empty: Option<P> = None;
    let at = &raw const empty as usize;
    let inserted = empty.get_or_insert(P { a: 3, b: 4 }) as *const P as usize;
    println!("get_or_insert {}", inserted - at);
    let again = empty.insert(P { a: 5, b: 6 }) as *const P as usize;
    println!("insert {}", again - at);

    let mut wrapped = (None::<P>, 1u8);
    let wa = &raw const wrapped.0 as usize;
    let wi = wrapped.0.get_or_insert(P { a: 20, b: 21 }) as *const P as usize;
    println!("inserted in a tuple {} {}", wi - wa, wrapped.1);
    let mut slots: [Option<P>; 2] = [None, None];
    let sa = &raw const slots as usize;
    let si = slots[1].get_or_insert(P { a: 22, b: 23 }) as *const P as usize;
    println!("inserted in an array {}", si - sa);
    let pair = (Some(P { a: 7, b: 8 }), 9u8);
    let pb = &raw const pair as usize;
    println!("in a tuple {}", pair.0.as_ref().unwrap() as *const P as usize - pb);

    let slot = Slot { tag: 1, held: Some(P { a: 10, b: 11 }) };
    let sb = &raw const slot as usize;
    let lent = slot.held.as_ref().unwrap() as *const P as usize;
    println!("in a field {} {}", &raw const slot.held as usize - sb, lent - sb);
    println!("the field by reference {}", outer(&slot.held) - sb);

    let cell = RefCell::new(P { a: 12, b: 13 });
    let value = &*cell.borrow() as *const P as usize;
    println!("cell {}", value - of_cell(&cell));

    println!("{:?} {:?} {:?} {} {:?}", o, empty, pair, slot.tag, slot.held);
}
