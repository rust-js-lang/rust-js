// A loop's value assigned in an `if`'s test, then another loop's value in
// its `else`: the second loop's way out is to a block inside the first, so
// it stays where it is (fuzz seed 12448939).
fn id<T>(x: T) -> T {
    x
}

fn main() {
    let mut v22: u8 = id(0u8);
    println!(
        "{:<4}|",
        (if ({
            v22 = ({
                let mut c900595: u32 = 0;
                loop {
                    c900595 += 1;
                    if c900595 > id(2u32) || id(false) {
                        break v22;
                    }
                }
            });
            id(false)
        }) {
            id(0u64)
        } else {
            ({
                let mut c421536: u32 = 0;
                loop {
                    c421536 += 1;
                    if c421536 > id(2u32) || id(true) {
                        break id(0u64);
                    }
                }
            })
        })
    );
}
