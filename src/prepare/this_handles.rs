//! A handle on a place of its function's `this`, `this._count`, kept to
//! the end, which preparation writes as an object whose getter and setter
//! read the place: `this` read through `const this$ = this;`, as theirs
//! is that object (#449). A class's member's is its instance's name
//! already, `const counted = this;` (`classes::this_outside_accessors`);
//! any other function's, `js::function::with_this`'s say, is `this$`, as
//! a function nested in it reads its `this` (ADR 0972).

use crate::js::{self, Stmt};

/// Of a function's `body`, before its handles are written.
pub(super) fn this_read_apart(body: &mut Vec<Stmt>) {
    js::this_outside_accessors(body, "this$");
}
