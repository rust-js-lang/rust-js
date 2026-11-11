// More than one array copied, and stepped over, through a pointer to part
// of a longer array cast to an array's type: overlapping forward and back,
// apart, and a `Copy` struct's arrays (#1353).
#[derive(Clone, Copy, Debug)]
struct P {
    x: u8,
    y: i16,
}

fn main() {
    let mut k: [u32; 6] = [0, 1, 2, 3, 4, 5];
    unsafe {
        let p = k.as_mut_ptr();
        std::ptr::copy(p.cast::<[u32; 2]>(), p.add(1).cast::<[u32; 2]>(), 2);
    }
    println!("{:?}", k);

    let mut back: [u32; 7] = [0, 1, 2, 3, 4, 5, 6];
    unsafe {
        let p = back.as_mut_ptr();
        std::ptr::copy(p.add(3).cast::<[u32; 2]>(), p.add(1).cast::<[u32; 2]>(), 2);
    }
    println!("{:?}", back);

    let from = [10u64, 11, 12, 13, 14, 15];
    let mut to = [0u64; 8];
    unsafe {
        let src = from.as_ptr().cast::<[u64; 3]>();
        let dst = to.as_mut_ptr().add(1).cast::<[u64; 3]>();
        std::ptr::copy_nonoverlapping(src, dst, 2);
        println!("{:?}", *src.add(1));
    }
    println!("{:?}", to);

    let mut ps = [P { x: 1, y: -1 }, P { x: 2, y: -2 }, P { x: 3, y: -3 }, P { x: 4, y: -4 }, P { x: 5, y: -5 }];
    unsafe {
        let p = ps.as_mut_ptr();
        std::ptr::copy(p.cast::<[P; 2]>(), p.add(1).cast::<[P; 2]>(), 2);
        (*p.cast::<[P; 2]>().add(1))[0].x = 9;
    }
    ps[1].y = 7;
    ps[3].x = 8;
    println!("{:?}", ps);
}
