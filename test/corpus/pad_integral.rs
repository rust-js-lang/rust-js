// `f.pad_integral(non_negative, prefix, digits)`, as rust_decimal shows a
// decimal: the digits given a sign, the prefix where `{:#}` asks, and the
// `Formatter`'s options as a number's.

use std::fmt;

struct Fixed {
    negative: bool,
    digits: String,
}

impl fmt::Display for Fixed {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.pad_integral(!self.negative, "0d", &self.digits)
    }
}

fn main() {
    let up = Fixed {
        negative: false,
        digits: "12.50".into(),
    };
    let down = Fixed {
        negative: true,
        digits: "3.25".into(),
    };
    println!("[{up}] [{down}] [{up:+}] [{down:+}]");
    println!("[{up:>8}] [{down:<8}] [{up:^9}] [{down:08}]");
    println!("[{up:#}] [{down:#010}] [{up:*>+9}]");
}
