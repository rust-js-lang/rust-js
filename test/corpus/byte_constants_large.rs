// A byte constant past 16 KiB is base64 text a runtime helper decodes,
// `$bytes(..)`, where a smaller one is a literal of each byte (#1096): read
// each way, both are the array of numbers they were, as native Rust's.
const AT_THE_LIMIT: &[u8; 16384] = &made::<16384>();
const PAST_THE_LIMIT: &[u8; 16385] = &made::<16385>();
const SLICE_PAST: &[u8] = &made::<20000>();

const fn made<const N: usize>() -> [u8; N] {
    let mut bytes = [0u8; N];
    let mut i = 0;
    while i < N {
        bytes[i] = (i * 7 % 251) as u8;
        i += 1;
    }
    bytes
}

fn sum<T: AsRef<[u8]>>(bytes: T) -> u64 {
    bytes.as_ref().iter().map(|&b| u64::from(b)).sum()
}

fn report(name: &str, bytes: &[u8]) {
    println!(
        "{name}: {} {} {} {:?} {:?} {}",
        bytes.len(),
        bytes[0],
        bytes[bytes.len() - 1],
        &bytes[..4],
        bytes.iter().rev().take(3).collect::<Vec<_>>(),
        sum(bytes),
    );
}

fn main() {
    report("at", AT_THE_LIMIT);
    report("past", PAST_THE_LIMIT);
    report("slice", SLICE_PAST);
    // One a body writes in place, as `include_bytes!` is.
    report("inline", const { &made::<17000>() });
    let mut copy = PAST_THE_LIMIT.to_vec();
    copy[0] = 99;
    println!("{} {} {}", copy[0], PAST_THE_LIMIT[0], copy == PAST_THE_LIMIT.to_vec());
    println!("{}", &PAST_THE_LIMIT[..16384] == AT_THE_LIMIT);
    println!("{}", sum(*PAST_THE_LIMIT) == sum(&PAST_THE_LIMIT[..]));
    let inline = b"short bytes";
    println!("{:?} {}", inline, sum(inline));
}
