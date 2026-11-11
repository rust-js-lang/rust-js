// Memory moved over memory it overlaps, as std defines each move (#1349):
// `ptr::swap` of two `[T; W]`s a few items apart reads one whole before
// it writes, `ptr::copy` is memmove's, `copy_within` and `rotate_*` keep
// what they move, and `ptr::replace` gives what was there; for items of
// 1, 4 and 16 bytes, runs of 2, 3, 4 and 7, every distance from 0 to the
// run, each way. `swap_nonoverlapping` of runs at least `W` apart is the
// control, which swapped two whole arrays' views wrongly too (#1350); and
// one `[T; 2]` of what isn't `Copy` copied over one it overlaps.
use std::fmt::Debug;
use std::ptr;

const LEN: usize = 21;

fn start<T: From<u8>>() -> [T; LEN] {
    std::array::from_fn(|i| T::from(i as u8))
}

fn overlaps<T: Copy + Debug + From<u8>, const W: usize>(name: &str) {
    for k in 0..=W {
        // `ptr::swap` of two `[T; W]`s `k` items apart, each way.
        let mut a: [T; LEN] = start();
        unsafe {
            let p = a.as_mut_ptr();
            ptr::swap(p.cast::<[T; W]>(), p.add(k).cast::<[T; W]>());
        }
        println!("{name} {W} {k} swap {:?}", a);
        let mut a: [T; LEN] = start();
        unsafe {
            let p = a.as_mut_ptr();
            ptr::swap(p.add(k).cast::<[T; W]>(), p.cast::<[T; W]>());
        }
        println!("{name} {W} {k} swap back {:?}", a);
        // `ptr::copy`, memmove's, forward and back.
        let mut a: [T; LEN] = start();
        unsafe {
            let p = a.as_mut_ptr();
            ptr::copy(p, p.add(k), W);
        }
        println!("{name} {W} {k} copy {:?}", a);
        let mut a: [T; LEN] = start();
        unsafe {
            let p = a.as_mut_ptr();
            ptr::copy(p.add(k), p, W);
        }
        println!("{name} {W} {k} copy back {:?}", a);
        // A whole `[T; W]` copied over one `k` along.
        let mut a: [T; LEN] = start();
        unsafe {
            let p = a.as_mut_ptr();
            ptr::copy(p.cast::<[T; W]>(), p.add(k).cast::<[T; W]>(), 1);
        }
        println!("{name} {W} {k} copy whole {:?}", a);
        // `copy_within`, each way.
        let mut a: [T; LEN] = start();
        a.copy_within(0..W, k);
        println!("{name} {W} {k} copy_within {:?}", a);
        let mut a: [T; LEN] = start();
        a.copy_within(k..k + W, 0);
        println!("{name} {W} {k} copy_within back {:?}", a);
        // `rotate_left` and `rotate_right` of the first `W`.
        let mut a: [T; LEN] = start();
        a[..W].rotate_left(k);
        println!("{name} {W} {k} rotate_left {:?}", a);
        let mut a: [T; LEN] = start();
        a[..W].rotate_right(k);
        println!("{name} {W} {k} rotate_right {:?}", a);
        // A slice's `swap`, of an item with itself too.
        let mut a: [T; LEN] = start();
        a.swap(0, k);
        a.swap(k, k);
        println!("{name} {W} {k} slice swap {:?}", a);
        // `ptr::replace` of a `[T; W]` with one read `k` along.
        let mut a: [T; LEN] = start();
        let old = unsafe {
            let p = a.as_mut_ptr();
            let along = p.add(k).cast::<[T; W]>().read();
            ptr::replace(p.cast::<[T; W]>(), along)
        };
        println!("{name} {W} {k} replace {:?} {:?}", old, a);
        // The control: `swap_nonoverlapping` of `[T; W]`s at least `W` apart.
        let mut a: [T; LEN] = start();
        unsafe {
            let p = a.as_mut_ptr();
            ptr::swap_nonoverlapping(p.cast::<[T; W]>(), p.add(W + k).cast::<[T; W]>(), 1);
        }
        println!("{name} {W} {k} swap_nonoverlapping {:?}", a);
        let mut a: [T; LEN] = start();
        unsafe {
            let p = a.as_mut_ptr();
            ptr::swap_nonoverlapping(p, p.add(W + k), W);
        }
        println!("{name} {W} {k} swap_nonoverlapping items {:?}", a);
    }
}

fn each_width<T: Copy + Debug + From<u8>>(name: &str) {
    overlaps::<T, 2>(name);
    overlaps::<T, 3>(name);
    overlaps::<T, 4>(name);
    overlaps::<T, 7>(name);
}

// What isn't `Copy` and has no drop, copied whole over a run it overlaps:
// no copy of each item is made, so the run is read whole first.
#[derive(Debug)]
struct Tag {
    n: u32,
}

fn tags_copied_over_themselves() {
    for k in 0..=2 {
        let mut tags: [Tag; 5] = std::array::from_fn(|n| Tag { n: n as u32 });
        unsafe {
            let p = tags.as_mut_ptr();
            ptr::copy(p.cast::<[Tag; 2]>(), p.add(k).cast::<[Tag; 2]>(), 1);
        }
        println!("tags {k} {tags:?}");
        let mut tags: [Tag; 5] = std::array::from_fn(|n| Tag { n: n as u32 });
        unsafe {
            let p = tags.as_mut_ptr();
            ptr::copy(p.add(k).cast::<[Tag; 2]>(), p.cast::<[Tag; 2]>(), 1);
        }
        println!("tags {k} back {tags:?}");
    }
}

// The issue's program, of each item's own type, where JS views a byte
// array's items as its bytes.
fn swapped_at_each_offset() {
    for offset in 0..3 {
        let mut bytes: [u8; 8] = std::array::from_fn(|i| i as u8);
        let mut words: [u32; 8] = std::array::from_fn(|i| i as u32);
        let mut wide: [u128; 8] = std::array::from_fn(|i| i as u128);
        unsafe {
            let p = bytes.as_mut_ptr();
            ptr::swap(p.cast::<[u8; 3]>(), p.add(offset).cast::<[u8; 3]>());
            let p = words.as_mut_ptr();
            ptr::swap(p.cast::<[u32; 2]>(), p.add(offset).cast::<[u32; 2]>());
            let p = wide.as_mut_ptr();
            ptr::swap(p.add(offset).cast::<[u128; 3]>(), p.cast::<[u128; 3]>());
        }
        println!("{offset} {bytes:?} {words:?} {wide:?}");
    }
}

// A whole run copied over one it overlaps, of each item's own type: a byte
// array's run is a view of its bytes, each of whose 4 is copied, not 1.
fn copied_whole_at_each_offset() {
    for offset in 0..4 {
        let mut bytes: [u8; 8] = std::array::from_fn(|i| i as u8);
        let mut words: [u32; 8] = std::array::from_fn(|i| i as u32);
        let mut wide: [u128; 8] = std::array::from_fn(|i| i as u128);
        unsafe {
            let p = bytes.as_mut_ptr();
            ptr::copy(p.add(offset).cast::<[u8; 4]>(), p.add(3).cast::<[u8; 4]>(), 1);
            let p = words.as_mut_ptr();
            ptr::copy(p.add(3).cast::<[u32; 4]>(), p.add(offset).cast::<[u32; 4]>(), 1);
            let p = wide.as_mut_ptr();
            ptr::copy(p.add(offset).cast::<[u128; 3]>(), p.add(2).cast::<[u128; 3]>(), 1);
        }
        println!("copied {offset} {bytes:?} {words:?} {wide:?}");
    }
}

fn main() {
    each_width::<u8>("u8");
    each_width::<u32>("u32");
    each_width::<u128>("u128");
    swapped_at_each_offset();
    copied_whole_at_each_offset();
    tags_copied_over_themselves();
}
