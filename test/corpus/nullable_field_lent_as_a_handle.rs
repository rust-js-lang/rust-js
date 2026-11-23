// A `&mut` to a `#[rust_js::nullable]` field lent on as a handle stores
// what's written through it as the field does, its `None` `null`, which
// a read of the field the crate alone writes tests `=== null`: given to a
// `&mut dyn FnMut`'s argument, given back by a function, kept in a
// vector, an item's field, a `RefCell` borrow's, a `Box`'s, and a `dyn`
// trait object's (#2427). And `get_or_insert`, `insert` and
// `get_or_insert_with` of one, which crashed the compiler (#2426).
use std::cell::RefCell;

struct Slot {
    #[cfg_attr(rust_js, rust_js::nullable)]
    v: Option<f64>,
}

fn dynamic(f: &mut dyn FnMut(&mut Option<f64>), o: &mut Option<f64>) {
    f(o)
}

fn generic<F: FnMut(&mut Option<f64>)>(mut f: F, o: &mut Option<f64>) {
    f(o)
}

fn kept<'a>(o: &'a mut Option<f64>) -> &'a mut Option<f64> {
    o
}

fn keep_in<'a>(into: &mut Vec<&'a mut Option<f64>>, o: &'a mut Option<f64>) {
    into.push(o);
}

trait Clear {
    fn clear(&mut self);
}

impl Clear for Option<f64> {
    fn clear(&mut self) {
        *self = None;
    }
}

fn main() {
    let mut s = Slot { v: Some(5.0) };
    dynamic(&mut |o| *o = None, &mut s.v);
    println!("dyn {} {:?}", s.v.is_none(), s.v);
    s.v = Some(1.0);
    generic(|o| *o = None, &mut s.v);
    println!("generic {} {:?}", s.v.is_none(), s.v);
    s.v = Some(1.0);
    *kept(&mut s.v) = None;
    println!("kept {} {:?}", s.v.is_none(), s.v);
    s.v = Some(1.0);
    {
        let mut held = Vec::new();
        keep_in(&mut held, &mut s.v);
        *held[0] = None;
    }
    println!("keep_in {} {:?}", s.v.is_none(), s.v);
    let mut es = vec![Slot { v: Some(1.0) }, Slot { v: Some(2.0) }];
    dynamic(&mut |o| *o = None, &mut es[1].v);
    println!("item dyn {} {:?}", es[1].v.is_none(), es[1].v);
    let i = es.len() - 1;
    es[i].v = Some(3.0);
    *kept(&mut es[i].v) = None;
    println!("item kept {} {:?}", es[i].v.is_none(), es[i].v);
    s.v = Some(1.0);
    let d: &mut dyn Clear = &mut s.v;
    d.clear();
    println!("dyn trait {} {:?}", s.v.is_none(), s.v);
    let r = RefCell::new(Slot { v: Some(1.0) });
    dynamic(&mut |o| *o = None, &mut r.borrow_mut().v);
    println!("refcell {} {:?}", r.borrow().v.is_none(), r.borrow().v);
    let mut z = Slot { v: None };
    let got = z.v.get_or_insert(10.0);
    *got += 1.0;
    println!("get_or_insert {} {:?}", z.v.is_none(), z.v);
    let mut y = Slot { v: Some(1.0) };
    *y.v.insert(2.0) += 1.0;
    let with = y.v.get_or_insert_with(|| 7.0);
    *with += 1.0;
    println!("insert {:?}", y.v);
    y.v = None;
    println!("{}", y.v.get_or_insert_with(|| 4.0));
    println!("get_or_insert_with {} {:?}", y.v.is_none(), y.v);
    let mut b = Box::new(Slot { v: Some(1.0) });
    dynamic(&mut |o| *o = None, &mut b.v);
    println!("boxed {} {:?}", b.v.is_none(), b.v);
}
