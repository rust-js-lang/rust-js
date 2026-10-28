//! An object literal as a person writes it: a dictionary's entries written
//! out, its literal (ADR 0666), and a spread of an object written out, its
//! fields (ADR 0967, amended).

use crate::js::{self, Expr, ExprKind, ObjectLayout, Prop};

/// `Object.fromEntries([["/index.html", code], ..])`, each key a string
/// written out, is the object literal of them, `{ "/index.html": code }`:
/// the same properties, made in the same order. Not of a key given twice,
/// which a literal would have twice, nor of `__proto__`, which a literal
/// makes the prototype, nor of an `undefined` value, which an object's
/// field leaves out (ADR 0280) and an entry keeps.
pub(super) fn entries_written_out(e: &Expr) -> Option<Expr> {
    let ExprKind::Call(f, args) = &e.kind else {
        return None;
    };
    let (ExprKind::Member(object, name), [entries]) = (&f.kind, args.as_slice()) else {
        return None;
    };
    let (ExprKind::Var(object), ExprKind::Array(entries)) = (&object.kind, &entries.kind) else {
        return None;
    };
    if object != "Object" || name != "fromEntries" {
        return None;
    }
    let mut fields = Vec::new();
    for entry in entries {
        let ExprKind::Array(pair) = &entry.kind else {
            return None;
        };
        let [key, value] = pair.as_slice() else {
            return None;
        };
        let ExprKind::Str(key) = &key.kind else {
            return None;
        };
        let given = fields
            .iter()
            .any(|field| matches!(field, Prop::Field(k, _) if k == key));
        if given || key == "__proto__" || matches!(value.kind, ExprKind::Undefined) {
            return None;
        }
        fields.push(Prop::Field(key.clone(), value.clone()));
    }
    Some(Expr {
        facts: js::Facts::NONE,
        kind: ExprKind::Object(fields, js::ObjectLayout::default()),
        span: e.span,
    })
}

/// A spread of an object written out, `{ ...EventInterface, ...{ view: 0 }
/// }`, is its fields, `{ ...EventInterface, view: 0 }`, as React writes a
/// `Dict` spread before more of it (ADR 0967, amended): the object written
/// out is new, so spreading it defines each of its fields where the outer
/// object would, in order, each made where it was, a later one's value
/// kept where an earlier one's key is; a `__proto__` field too, printed
/// `["__proto__"]`, its own as the spread's copy is. Not one of a getter or
/// a setter, whose value a spread copies, where the object's field would
/// be the getter. Its layout is the object's, across lines where it was.
pub(super) fn spread_written_out(props: &mut Vec<Prop>, layout: &mut ObjectLayout) {
    let written_out = |prop: &Prop| match prop {
        Prop::Spread(Expr {
            kind: ExprKind::Object(inner, _),
            ..
        }) => inner
            .iter()
            .all(|p| matches!(p, Prop::Field(..) | Prop::Method(..) | Prop::Spread(_))),
        _ => false,
    };
    if !props.iter().any(written_out) {
        return;
    }
    let mut flat = Vec::with_capacity(props.len());
    for prop in std::mem::take(props) {
        match (written_out(&prop), prop) {
            (
                true,
                Prop::Spread(Expr {
                    kind: ExprKind::Object(inner, inner_layout),
                    ..
                }),
            ) => {
                layout.lines |= inner_layout.lines;
                layout.kept.extend(inner_layout.kept);
                layout.led.extend(inner_layout.led);
                if let Some(docs) = inner_layout.docs {
                    layout.docs.get_or_insert_default().0.extend(docs.0);
                }
                flat.extend(inner);
            }
            (_, prop) => flat.push(prop),
        }
    }
    *props = flat;
}
