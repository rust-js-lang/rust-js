// `MaybeUninit::zeroed()` of an array of `MaybeUninit<T>`s, each its `T`'s
// zero, of a length generic code is given, `[MaybeUninit<u8>; CAP]`,
// arrayvec's `ArrayString::zero_filled`, or a known one (ADR 0332). And
// one not yet written, `uninit().assume_init()`, as many items not yet
// written, arrayvec's `ArrayVec::new`.
use std::mem::MaybeUninit;

struct Bytes<const CAP: usize> {
    xs: [MaybeUninit<u8>; CAP],
    len: usize,
}

impl<const CAP: usize> Bytes<CAP> {
    fn zeroed() -> Self {
        unsafe {
            Bytes {
                xs: MaybeUninit::zeroed().assume_init(),
                len: 0,
            }
        }
    }

    fn uninit() -> Self {
        unsafe {
            Bytes {
                xs: MaybeUninit::uninit().assume_init(),
                len: 0,
            }
        }
    }

    fn sum(&self) -> u32 {
        self.xs.iter().map(|b| unsafe { b.assume_init() } as u32).sum()
    }
}

fn main() {
    let b: Bytes<4> = Bytes::zeroed();
    let mut u: Bytes<3> = Bytes::uninit();
    u.xs[u.len].write(7);
    u.len += 1;
    println!("{} {}", u.xs.len(), unsafe { u.xs[0].assume_init() });
    let fixed: [MaybeUninit<u16>; 3] = unsafe { MaybeUninit::zeroed().assume_init() };
    println!("{} {} {}", b.sum(), b.len, unsafe { fixed[2].assume_init() });
}
