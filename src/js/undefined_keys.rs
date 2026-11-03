//! An object's fields that are `undefined`, a `None` say, left out, `{ code }`,
//! as JS leaves them out: what reads one reads `undefined` either way (ADR
//! 0280).

use super::{ExprKind, ObjectLayout, Prop};

/// `props` of an object written as `layout` says, without its keys of
/// `undefined`. Not after a spread, whose field it would then be, nor one a
/// comment is written after on its line, the comment's place, but where the
/// field is optional, absent until set (amended, 2026-11-03); a
/// comment on lines of its own before one is before the next key, or the
/// object's `}` (amended again, rd321): it changes no key. Nor a field that
/// holds a cell, set later, `destroy: undefined`, the object's shape whole
/// from its start (amended, the user's decision, 2026-10-27).
pub fn left_out(props: &mut Vec<Prop>, layout: &ObjectLayout) {
    if props.iter().any(|p| matches!(p, Prop::Spread(_))) {
        return;
    }
    props.retain(|p| {
        !matches!(p, Prop::Field(key, value)
            if matches!(value.kind, ExprKind::Undefined) && !layout.kept.contains(key))
    });
}
