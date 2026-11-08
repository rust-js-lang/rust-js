//! A library's impl of a std trait for `String`, which its own code never
//! asks for, so its consumer compiles it from its MIR: `push` and
//! `push_str` through its `&mut self`, a cell, write its `value` (#1026).
use std::ops::AddAssign;

pub struct Tag(pub &'static str);

impl AddAssign<Tag> for String {
    fn add_assign(&mut self, t: Tag) {
        self.push('<');
        self.push_str(t.0);
        self.push('>');
    }
}
