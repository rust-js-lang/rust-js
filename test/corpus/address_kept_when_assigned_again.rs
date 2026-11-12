//@ native: wasm32
// A place a raw pointer is taken to or into, assigned whole again, keeps
// its address, and the old pointer reads what it holds now (#1484): JS's
// `p = v` would make the variable another object, at another address, so
// it's written over what it holds, `$assign(p, v)`. A struct, a tuple, an
// array, a `Result`, a field, a whole of a field a pointer is into (that
// field's object kept, written over first), a call's result, in a loop,
// and a value with drop glue, dropped at the assignment as Rust drops it,
// a `Vec` too, an array's item; an `Option` and a `Box`, cells already. A
// `Copy` value read out before the write keeps what it read.
#[derive(Debug, Clone, Copy, PartialEq)]
struct P {
    a: u8,
    b: u32,
}

// Changed in place by nothing but the write over what holds it.
#[derive(Debug, Clone, Copy)]
struct R(u8, u16);

#[derive(Debug)]
struct T {
    r: R,
}

struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Debug)]
struct S {
    f: P,
    n: u32,
}

fn make(n: u32) -> P {
    P { a: n as u8, b: n }
}

fn main() {
    // A struct, read through the old pointer after each reassignment.
    let mut p = P { a: 1, b: 2 };
    let at = &raw const p;
    p = P { a: 3, b: 4 };
    println!("struct {} {:?}", at == &raw const p, unsafe { *at });
    // A tuple.
    let mut t = (1u8, 2u32);
    let at = &raw const t;
    t = (5, 6);
    println!("tuple {} {:?}", at == &raw const t, unsafe { *at });
    // An array.
    let mut a = [1u16, 2, 3];
    let at = &raw const a;
    a = [7, 8, 9];
    println!("array {} {:?}", at == &raw const a, unsafe { *at });
    // An Option of an object.
    let mut o = Some(P { a: 1, b: 2 });
    let at = &raw const o;
    o = None;
    println!("option {} {:?}", at == &raw const o, unsafe { *at });
    o = Some(P { a: 9, b: 9 });
    println!("option {} {:?}", at == &raw const o, unsafe { *at });
    // A Result, an enum.
    let mut r: Result<u32, u8> = Ok(1);
    let at = &raw const r;
    r = Err(2);
    println!("result {} {:?}", at == &raw const r, unsafe { *at });
    // A Box: the local holds the pointer, so its address is the local's.
    let mut b = Box::new(P { a: 1, b: 2 });
    let at = &raw const b;
    b = Box::new(P { a: 4, b: 4 });
    println!("box {} {:?}", at == &raw const b, unsafe { &**at });
    // Through a field.
    let mut s = S {
        f: P { a: 1, b: 2 },
        n: 0,
    };
    let at = &raw const s.f;
    s.f = P { a: 6, b: 7 };
    println!("field {} {:?} {}", at == &raw const s.f, unsafe { *at }, s.n);
    // Drop order: the old value drops at the assignment.
    let mut d = (D(1), 0u8);
    let at = &raw const d;
    println!("before");
    d = (D(2), 1);
    println!("after {} {}", at == &raw const d, unsafe { (*at).0.0 });
    // A moved-from local assigned again: the value moved keeps its own.
    let mut m = P { a: 1, b: 1 };
    let at = &raw const m;
    let kept = m;
    m = P { a: 2, b: 2 };
    println!("moved {} {:?} {:?}", at == &raw const m, kept, m);
    // The whole, a pointer into a field of it taken.
    let at = &raw const s.f.b;
    s = S {
        f: P { a: 8, b: 9 },
        n: 1,
    };
    println!("whole {} {} {:?}", at == &raw const s.f.b, unsafe { *at }, s);
    // A call's result.
    let mut p = make(1);
    let at = &raw const p;
    p = make(2);
    println!("call {} {:?}", at == &raw const p, unsafe { *at });
    // In a loop.
    let mut q = make(0);
    let at = &raw const q;
    for i in 1..3 {
        q = make(i);
        println!("loop {} {:?}", at == &raw const q, unsafe { *at });
    }
    // A `Copy` value read out of an object kept inside a place written
    // over, before the write: a copy, which keeps what it read.
    let mut t2 = T { r: R(1, 2) };
    let at = &raw const t2.r.1;
    let seen = t2.r;
    t2 = T { r: R(3, 4) };
    println!("copied {:?} {} {}", seen, unsafe { *at }, at == &raw const t2.r.1);
    // An item of an array, a field of it a pointer is into.
    let mut items = [S {
        f: P { a: 1, b: 1 },
        n: 0,
    }];
    let at = &raw const items[0].f.b;
    items[0] = S {
        f: P { a: 2, b: 3 },
        n: 4,
    };
    println!(
        "item {} {} {:?}",
        at == &raw const items[0].f.b,
        unsafe { *at },
        items[0]
    );
    // Dropped at each assignment, in a loop.
    let mut e = D(1);
    let at = &raw const e;
    for i in 2..4 {
        e = D(i);
        println!("now {} {}", at == &raw const e, unsafe { (*at).0 });
    }
    let mut v = vec![D(10)];
    let at = &raw const v;
    v = vec![D(11), D(12)];
    println!("vec {} {}", at == &raw const v, unsafe { (*at).len() });
}
