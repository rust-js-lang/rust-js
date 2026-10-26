//! An object's fields that are `undefined`, a `None` say, left out, `{ code }`,
//! as JS leaves them out: what reads one reads `undefined` either way (ADR
//! 0280).

use super::{ExprKind, ObjectLayout, Prop};

/// `props` of an object written as `layout` says, without its keys of
/// `undefined`. Not after a spread, whose field it would then be, nor one a
/// comment is written after on its line, the comment's place (amended); one
/// a comment is before only where no key is kept after it, before which the
/// comment is otherwise (amended again): a comment changes which keys are
/// written only where it has no other place.
pub fn left_out(props: &mut Vec<Prop>, layout: &ObjectLayout) {
    if props.iter().any(|p| matches!(p, Prop::Spread(_))) {
        return;
    }
    let undefined = |p: &Prop| {
        matches!(p, Prop::Field(key, value)
            if matches!(value.kind, ExprKind::Undefined) && !layout.commented.contains(key))
    };
    let mut kept_after = false;
    let kept: Vec<bool> = (props.iter().rev())
        .map(|p| {
            let led = matches!(p, Prop::Field(key, _) if layout.led.contains(key));
            let kept = !undefined(p) || (led && !kept_after);
            kept_after |= kept;
            kept
        })
        .collect();
    let mut kept = kept.into_iter().rev();
    props.retain(|_| kept.next().unwrap_or(true));
}
