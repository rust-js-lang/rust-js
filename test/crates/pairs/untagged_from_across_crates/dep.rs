// A library's `From`s into its untagged enum (#1775, ADR 0214, amended):
// `From<String>`, the variant of its argument, is the value it's given,
// as its contract lists it among what gives its argument; the others,
// which own what they're given, print or choose a variant, are called.

#[cfg_attr(rust_js, rust_js::untagged)]
#[derive(Debug, PartialEq)]
pub enum Detail {
    Text(String),
    Number(u32),
}

impl From<&str> for Detail {
    fn from(value: &str) -> Self {
        Self::Text(value.to_owned())
    }
}

impl From<u32> for Detail {
    fn from(value: u32) -> Self {
        println!("made of {value}");
        Self::Number(value + 2)
    }
}

impl From<String> for Detail {
    fn from(value: String) -> Self {
        Detail::Text(value)
    }
}

impl From<i64> for Detail {
    fn from(value: i64) -> Self {
        if value < 0 {
            Detail::Text(format!("minus {}", -value))
        } else {
            Detail::Number(value as u32)
        }
    }
}

pub fn detail<T: Into<Detail>>(value: T) -> Detail {
    value.into()
}

pub fn show(detail: &Detail) -> String {
    match detail {
        Detail::Text(text) => text.clone(),
        Detail::Number(n) => n.to_string(),
    }
}
