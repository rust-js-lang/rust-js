// What's left of a packed struct's part, one field of which moved out,
// dropped where the struct ends: rustc moves a part that may be misaligned
// into a temporary of its own to drop it (`AddMovesForPackedDrops`), which
// the comparison with rustc's drop elaboration reads as the part, as
// rustc's packed/misaligned-reference-drop-field-99838.rs drops it (#2112).
struct Loud(&'static str);

impl Drop for Loud {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Wrapper {
    first: Loud,
    second: Loud,
}

#[repr(packed)]
struct Misaligned(u8, Wrapper);

fn main() {
    let packed = Misaligned(
        0,
        Wrapper {
            first: Loud("first"),
            second: Loud("second"),
        },
    );
    // Somewhere even, so the part is at an odd address natively.
    let held: ([u16; 0], Misaligned) = ([], packed);
    let second = held.1.1.second;
    println!("moved {}", second.0);
    let whole = Misaligned(
        1,
        Wrapper {
            first: Loud("whole first"),
            second: Loud("whole second"),
        },
    );
    let tag = whole.0;
    println!("tag {tag}");
}
