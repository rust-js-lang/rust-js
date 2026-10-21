// `{:x?}` of a composite shows every integer inside it in hex, as Rust's
// `Debug` passes the flag down (ADR 0559): an array, a `Vec`, a tuple, an
// `Option`, and a derived struct and enum, `{:#x?}` pretty too.
#[derive(Debug)]
struct Header {
    id: u32,
    flags: u8,
    delta: i8,
    name: &'static str,
}

#[derive(Debug)]
enum Packet {
    Ping(u16),
    Data { len: u32, body: Vec<u8> },
}

fn main() {
    println!("{:x?}", [10u32, 255]);
    println!("{:X?}", vec![171u8, 205]);
    println!("{:x?}", (16u16, -1i8, "t"));
    println!("{:x?} {:x?}", Some(255u64), None::<u8>);
    let header = Header { id: 3054, flags: 0xf0, delta: -2, name: "h" };
    println!("{header:x?}");
    println!("{:x?}", Packet::Data { len: 32, body: vec![1, 254] });
    println!("{:x?} {:X?}", Packet::Ping(4096), Packet::Ping(0xabc));
    println!("{header:#x?}");
}
