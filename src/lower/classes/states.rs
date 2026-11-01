//! A class's state flattened into it (rd380, ADR 1044): its constructor
//! takes the state's keys, and each `new` gives them.

use super::{Member, class_of, member};
use crate::js::{self, Expr, ExprKind, Pattern, Prop, StmtKind};
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::def_id::DefId;

/// Of `ctor`, a class's constructor, its parameter that's the state its
/// class flattens (rd380, ADR 1044), `fn new(state: ChunkState)`: where it
/// is, and the state's keys, which the constructor takes in its place,
/// `function ReactPromise(status, value, reason)`, as each `new` gives
/// them, `new ReactPromise("pending", null, null)`. The one answer the
/// constructor and each call of it read.
pub(crate) fn flattened_parameter(tcx: TyCtxt<'_>, ctor: DefId) -> Option<(usize, Vec<String>)> {
    if member(tcx, ctor) != Some(Member::Constructor) {
        return None;
    }
    let class = tcx.adt_def(class_of(tcx, ctor)?);
    let (state, keys) = (class.non_enum_variant().fields.iter())
        .filter(|field| crate::lower::bindings::is_flatten(tcx, field))
        .find_map(|field| {
            let ty = tcx.type_of(field.did).instantiate_identity().skip_normalization();
            crate::lower::bindings::flattened_state(tcx, ty)
        })?;
    let inputs = tcx
        .fn_sig(ctor)
        .instantiate_identity()
        .skip_normalization()
        .skip_binder()
        .inputs()
        .to_vec();
    let at = inputs
        .iter()
        .position(|ty| matches!(ty.kind(), ty::Adt(of, _) if of.did() == state.did()))?;
    Some((at, keys))
}

/// A constructor's state parameter (`flattened_parameter`) as the state's
/// keys: each a parameter of its name, and the state's spread in the
/// instance's literal, `{ ...state }`, each key set of it, `this.status =
/// status;`. `Err` where the body reads the state but so.
pub(super) fn spread_state(tcx: TyCtxt<'_>, ctor: DefId, function: &mut js::Function) -> Result<(), &'static str> {
    let Some((at, keys)) = flattened_parameter(tcx, ctor) else {
        return Ok(());
    };
    let Some(Pattern::Name(state)) = function.params.get(at).cloned() else {
        return Err("a constructor whose flattened state is no parameter of its own name");
    };
    let mut taken = std::collections::HashSet::new();
    js::names_in(&function.body, &mut taken);
    for param in &function.params {
        if let Pattern::Name(name) = param {
            taken.insert(name.clone());
        }
    }
    taken.remove(&state);
    let names: Vec<String> = (keys.iter())
        .map(|key| crate::lower::fresh_in(&mut taken, key))
        .collect();
    let mut spread = 0;
    js::each_expr_mut(&mut function.body, &mut |e| {
        if let ExprKind::Object(props, _) = &mut e.kind {
            let mut flat = Vec::with_capacity(props.len());
            for prop in std::mem::take(props) {
                match prop {
                    Prop::Spread(Expr {
                        kind: ExprKind::Var(v), ..
                    }) if v == state => {
                        spread += 1;
                        flat.extend(
                            (keys.iter().zip(&names)).map(|(key, name)| Prop::Field(key.clone(), Expr::var(name))),
                        );
                    }
                    prop => flat.push(prop),
                }
            }
            *props = flat;
        }
    });
    if spread != 1 || js::mentions_in(&function.body, &state) > 0 {
        return Err(
            "a constructor that reads its flattened state but to make its instance's: it's given as its keys, `function ReactPromise(status, value, reason)`",
        );
    }
    function.params.splice(at..=at, names.into_iter().map(Pattern::Name));
    Ok(())
}

/// `new C(..)` of a constructor whose parameter `at` is its class's
/// flattened state (`classes::flattened_parameter`, rd380, ADR 1044): the
/// state given as its keys, `new ReactPromise("pending", null, null)`. Of a
/// variant made there, each of its fields where its key is, a key it
/// hasn't `undefined`; of a value read alone, each key of it; else, or
/// where its fields would be computed out of the order they're written in,
/// the value given once to a function that makes it, as it's computed once.
pub(crate) fn new_of_state(class: Expr, mut args: Vec<Expr>, at: usize, keys: &[String]) -> Expr {
    let state = args.remove(at);
    let given: Option<Vec<Expr>> = match &state.kind {
        ExprKind::Object(props, _) => {
            let field = |key: &str| {
                (props.iter()).find_map(|prop| match prop {
                    Prop::Field(name, value) if name == key => Some(value.clone()),
                    _ => None,
                })
            };
            let effects: Vec<usize> = (props.iter().enumerate())
                .filter(|(_, prop)| matches!(prop, Prop::Field(_, value) if value.has_effects()))
                .filter_map(|(_, prop)| match prop {
                    Prop::Field(name, _) => keys.iter().position(|key| key == name),
                    _ => None,
                })
                .collect();
            let fields_only = (props.iter()).all(|prop| matches!(prop, Prop::Field(name, _) if keys.contains(name)));
            (fields_only && effects.windows(2).all(|w| w[0] < w[1])).then(|| {
                keys.iter()
                    .map(|key| field(key).unwrap_or_else(Expr::undefined))
                    .collect()
            })
        }
        _ if state.reads_same() && !state.has_effects() => {
            Some(keys.iter().map(|key| Expr::member(state.clone(), key)).collect())
        }
        _ => None,
    };
    match given {
        Some(given) => {
            args.splice(at..at, given);
            Expr::new_(class, args)
        }
        None => {
            // Each argument a parameter, so each is computed in its order.
            args.insert(at, state);
            let names: Vec<String> = (0..args.len())
                .map(|i| if i == at { "state".into() } else { format!("arg{i}") })
                .collect();
            let mut inner: Vec<Expr> = names.iter().map(|name| Expr::var(name)).collect();
            inner.splice(at..=at, keys.iter().map(|key| Expr::member(Expr::var("state"), key)));
            let made = Expr::arrow(
                names.iter().map(|name| name.as_str().into()).collect(),
                vec![StmtKind::Return(Some(Expr::new_(class, inner))).at(js::Span::NONE)],
            );
            Expr::call(made, args)
        }
    }
}
