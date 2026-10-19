// `f.pad_integral(..)` where nothing in the crate formats with options: a
// number's digits and its sign, as written.

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
    println!("{up} {down}");
}
