// Whole values moved through raw pointers to what JS holds as an object
// itself, a `&mut`'s or a whole array's: `ptr::swap`, `ptr::replace` and
// `swap_nonoverlapping` give each place the other's value (#1350);
// `ptr::read` takes its bits apart from a place written after (#1351);
// and `ptr::write` or `*p = v` through a `&mut` made a pointer writes the
// caller's value, not a name of its own (#1352). The hand-written `take`
// and `swap` of unsafe code, as std's `mem::take` and `mem::swap` are.
use std::ptr;

#[derive(Debug, Clone, Copy)]
struct Point {
    x: u32,
    y: u32,
}

#[derive(Debug, Default)]
struct Named {
    name: String,
    items: Vec<u32>,
}

fn take_vec(v: &mut Vec<u32>) -> Vec<u32> {
    unsafe {
        let old = ptr::read(v);
        ptr::write(v, Vec::new());
        old
    }
}

fn take_named(n: &mut Named) -> Named {
    unsafe {
        let old = ptr::read(n);
        ptr::write(n, Named::default());
        old
    }
}

fn push_through(slot: &mut Vec<u32>) {
    unsafe {
        let mut old = ptr::read(slot);
        old.push(5);
        ptr::write(slot, old);
    }
}

fn swap_vecs(a: &mut Vec<u32>, b: &mut Vec<u32>) {
    unsafe {
        let t = ptr::read(a);
        ptr::copy_nonoverlapping(b, a, 1);
        ptr::write(b, t);
    }
}

fn swap_named(a: &mut Named, b: &mut Named) {
    unsafe {
        let t = ptr::read(a);
        ptr::copy_nonoverlapping(b, a, 1);
        ptr::write(b, t);
    }
}

fn swap_arrays(a: &mut [u32; 2], b: &mut [u32; 2]) {
    unsafe {
        let t = ptr::read(a);
        ptr::copy_nonoverlapping(b, a, 1);
        ptr::write(b, t);
    }
}

fn swapped() {
    let mut x = [1u32, 2];
    let mut y = [3u32, 4];
    unsafe { ptr::swap(&mut x as *mut [u32; 2], &mut y as *mut [u32; 2]) };
    println!("arrays swapped {x:?} {y:?}");
    let mut s = Point { x: 1, y: 2 };
    let mut t = Point { x: 3, y: 4 };
    unsafe { ptr::swap(&mut s as *mut Point, &mut t as *mut Point) };
    println!("points swapped {s:?} {t:?}");
    let mut a = vec![1u32, 2];
    let mut b = vec![3u32];
    unsafe { ptr::swap(&mut a as *mut Vec<u32>, &mut b as *mut Vec<u32>) };
    println!("vecs swapped {a:?} {b:?}");
    let mut m = Named {
        name: "m".to_string(),
        items: vec![1],
    };
    let mut n = Named {
        name: "n".to_string(),
        items: vec![2, 3],
    };
    unsafe { ptr::swap(&mut m as *mut Named, &mut n as *mut Named) };
    println!("named swapped {m:?} {n:?}");
    let (mut e, mut f) = ([1u32, 2, 3], [4u32, 5, 6]);
    let (re, rf) = (&mut e, &mut f);
    unsafe { ptr::swap(re, rf) };
    println!("references swapped {e:?} {f:?}");
    let mut g = [1u32, 2];
    let mut h = [3u32, 4];
    unsafe { ptr::swap_nonoverlapping(&mut g as *mut [u32; 2], &mut h as *mut [u32; 2], 1) };
    println!("swapped apart {g:?} {h:?}");
    let mut d: [u32; 6] = [0, 1, 2, 3, 4, 5];
    unsafe {
        let p = d.as_mut_ptr();
        ptr::swap(p.cast::<[u32; 2]>(), p.add(3).cast::<[u32; 2]>());
    }
    println!("parts swapped {d:?}");
    let mut v = vec![vec![1u32], vec![2, 3]];
    unsafe {
        let p = v.as_mut_ptr();
        ptr::swap(p, p.add(1));
        ptr::swap(p, p);
    }
    println!("items swapped {v:?}");
}

fn replaced() {
    let mut b = [5u32, 6];
    let old = unsafe { ptr::replace(&mut b as *mut [u32; 2], [7, 8]) };
    println!("array replaced {old:?} {b:?}");
    let mut c = vec![5u32, 6];
    let old = unsafe { ptr::replace(&mut c as *mut Vec<u32>, vec![7]) };
    println!("vec replaced {old:?} {c:?}");
    let mut u = Named {
        name: "u".to_string(),
        items: vec![4],
    };
    let old = unsafe {
        ptr::replace(
            &mut u,
            Named {
                name: "w".to_string(),
                items: vec![],
            },
        )
    };
    println!("named replaced {old:?} {u:?}");
    let mut items = vec![vec![1u32], vec![2]];
    let old = unsafe { ptr::replace(items.as_mut_ptr(), vec![9]) };
    println!("item replaced {old:?} {items:?}");
}

fn read_then_written() {
    let mut a = [1u32, 2];
    let p = &mut a as *mut [u32; 2];
    let old = unsafe { ptr::read(p) };
    unsafe { ptr::write(p, [3, 4]) };
    println!("array read {old:?} {a:?}");
    let mut z = vec![7u32];
    let p = &raw mut z;
    unsafe {
        let old = p.read();
        p.write(vec![8, 9]);
        println!("vec read {old:?} {:?}", *p);
    }
    let mut w = Named {
        name: "w".to_string(),
        items: vec![9],
    };
    let q = &mut w as *mut Named;
    let old = unsafe { q.read() };
    unsafe {
        q.write(Named {
            name: "x".to_string(),
            items: vec![],
        })
    };
    println!("named read {old:?} {w:?}");
    let mut items = vec![vec![1u32], vec![2]];
    unsafe {
        let p = items.as_mut_ptr();
        let old = p.read();
        p.write(vec![5]);
        println!("item read {old:?} {items:?}");
    }
    let mut pair = (vec![1u32], 2u32);
    unsafe {
        let p = &raw mut pair;
        let old = ptr::read(p);
        ptr::write(p, (vec![], 3));
        println!("pair read {old:?} {pair:?}");
    }
    // Text, which no write changes in place, read out as itself.
    let text = String::from("kept");
    let read = unsafe { ptr::read(&text) };
    std::mem::forget(text);
    println!("text read {read}");
    let mut kept = vec![1u32];
    let through = &raw mut kept;
    unsafe {
        *(&raw mut kept) = vec![2, 3];
        println!("written in place {:?}", *through);
    }
}

fn main() {
    swapped();
    replaced();
    read_then_written();
    let mut v = vec![1u32, 2];
    let old = take_vec(&mut v);
    println!("taken {old:?} {v:?}");
    let mut n = Named {
        name: "n".to_string(),
        items: vec![3],
    };
    let old = take_named(&mut n);
    println!("taken {old:?} {n:?}");
    let mut w = vec![4u32];
    push_through(&mut w);
    println!("pushed through {w:?}");
    let mut a = vec![1u32];
    let mut b = vec![2u32, 3];
    swap_vecs(&mut a, &mut b);
    println!("swapped by hand {a:?} {b:?}");
    let mut s = Named {
        name: "s".to_string(),
        items: vec![1],
    };
    let mut t = Named {
        name: "t".to_string(),
        items: vec![],
    };
    swap_named(&mut s, &mut t);
    println!("swapped by hand {s:?} {t:?}");
    let mut x = [1u32, 2];
    let mut y = [3u32, 4];
    swap_arrays(&mut x, &mut y);
    println!("swapped by hand {x:?} {y:?}");
    let mut vs = vec![vec![1u32], vec![2]];
    let (first, rest) = vs.split_at_mut(1);
    swap_vecs(&mut first[0], &mut rest[0]);
    println!("swapped by hand {vs:?}");
}
