// A `static mut` is its module's variable (ADR 0096, amended): a `let`, or
// an `export let` that another module reads by its name, as ES's live
// binding, and sets by a `pub fn` of its module, as JS lets only that module
// set it. Each is read by value here; a shared reference to one is in
// static_mut_shared.rs, and a pointer to one in static_mut_pointers.rs.
#[derive(Clone, Copy, Debug)]
struct Stats {
    calls: u32,
    last: i64,
}

static mut COUNT: u32 = 0;
static mut STATS: Stats = Stats { calls: 0, last: -1 };
static mut SLOTS: [u8; 3] = [0; 3];
static mut WRAP: u8 = 250;

mod log {
    pub static mut LINES: usize = 0;

    pub fn line() -> usize {
        unsafe {
            LINES += 1;
            LINES
        }
    }

    pub fn set_lines(lines: usize) {
        unsafe {
            LINES = lines;
        }
    }
}

fn bump() -> u32 {
    unsafe {
        COUNT += 1;
        COUNT
    }
}

fn record(value: i64) {
    unsafe {
        STATS.calls += 1;
        STATS.last = value;
        SLOTS[(value as usize) % 3] = value as u8;
        WRAP = WRAP.wrapping_add(3);
    }
}

fn main() {
    let first = bump();
    let second = bump();
    println!("{} {} {}", first, second, unsafe { COUNT });
    record(4);
    record(8);
    let stats = unsafe { STATS };
    record(1);
    let (calls, last, slot) = unsafe { (STATS.calls, STATS.last, SLOTS[1]) };
    println!("{:?} {} {} {}", stats, calls, last, slot);
    let (slots, wrap) = unsafe { (SLOTS, WRAP) };
    println!("{:?} {}", slots, wrap);
    log::set_lines(10);
    let set = unsafe { log::LINES };
    println!("{} {} {}", set, log::line(), log::line());
    let read = unsafe { log::LINES };
    println!("{} {}", set, read);
}
