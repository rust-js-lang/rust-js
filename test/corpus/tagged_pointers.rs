// A pointer's low bits carry a tag, as once_cell's waiter queue and semver's
// identifier pack theirs (ADR 0562): the tagged pointer has the place's
// provenance and another address; masked back, it's the place again.

// once_cell: the state in the two low bits of a 4-aligned pointer.
const STATE_MASK: usize = 0x3;
const RUNNING: usize = 0x1;
const COMPLETE_PTR: *mut Waiter = 0x2 as *mut Waiter;

#[repr(align(4))]
struct Waiter {
    signaled: bool,
    count: u32,
}

fn tagged(waiter: *mut Waiter, state: usize) -> *mut Waiter {
    waiter.map_addr(|addr| (addr & !STATE_MASK) | state)
}

fn untagged(queue: *mut Waiter) -> (*mut Waiter, usize) {
    (queue.map_addr(|addr| addr & !STATE_MASK), queue.addr() & STATE_MASK)
}

// semver: a 1 rotated into the top bit of a 2-aligned heap pointer, so the
// representation is never ASCII, and back.
fn ptr_to_repr(original: *mut u8) -> *mut u8 {
    let modified = (original as usize | 1).rotate_right(1);
    let diff = modified.wrapping_sub(original as usize);
    original.wrapping_add(diff)
}

fn repr_to_ptr(modified: *mut u8) -> *mut u8 {
    let original = (modified as usize) << 1;
    let diff = original.wrapping_sub(modified as usize);
    modified.wrapping_add(diff)
}

fn main() {
    let mut waiter = Waiter { signaled: false, count: 7 };
    let me = &raw mut waiter;
    let queue = tagged(me, RUNNING);
    let (head, state) = untagged(queue);
    println!("{} {} {} {}", queue == me, head == me, state == RUNNING, queue.addr() & STATE_MASK);
    unsafe {
        (*head).signaled = true;
        (*head).count += 1;
    }
    println!("{} {} {}", waiter.signaled, waiter.count, untagged(COMPLETE_PTR).1);
    println!("{} {}", COMPLETE_PTR.is_null(), COMPLETE_PTR.addr());

    let heap = Box::into_raw(Box::new(0x4142u16)) as *mut u8;
    let repr = ptr_to_repr(heap);
    let top_bit = repr as usize >> (usize::BITS - 1);
    let original = repr_to_ptr(repr);
    println!("{} {} {}", top_bit, original == heap, repr == heap);
    println!("{}", unsafe { *original.cast::<u16>() });
    drop(unsafe { Box::from_raw(original.cast::<u16>()) });
}
