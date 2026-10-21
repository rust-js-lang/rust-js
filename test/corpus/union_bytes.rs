// A union's field read as another plain type's: the field it holds, as
// bytes, read as the other (ADR 0562), as `transmute` reads them (ADR 0408).

#[repr(C)]
union Bits {
    int: u32,
    float: f32,
    bytes: [u8; 4],
    halves: (u16, u16),
}

#[repr(C)]
union Wide {
    whole: u64,
    parts: [u32; 2],
    small: u8,
}

fn main() {
    let mut bits = Bits { int: 0x3f80_0000 };
    let as_float = unsafe { bits.float };
    let as_bytes = unsafe { bits.bytes };
    let as_halves = unsafe { bits.halves };
    println!("{as_float} {as_bytes:?} {as_halves:?}");

    bits.float = -2.5;
    println!("{:#x} {:?}", unsafe { bits.int }, unsafe { bits.bytes });
    bits.bytes = [1, 2, 3, 4];
    println!("{:#x} {:?}", unsafe { bits.int }, unsafe { bits.halves });

    let mut wide = Wide {
        whole: 0x1122_3344_5566_7788,
    };
    println!("{:?} {:#x}", unsafe { wide.parts }, unsafe { wide.small });
    wide.parts = [1, 2];
    println!("{:#x}", unsafe { wide.whole });
}
