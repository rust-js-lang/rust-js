//! What a function's JS takes and gives, as a JS caller calls it: its
//! `JsSignature`, lowering's answer (ADR 1091). Each part is its owner's:
//! what its `self` is (`classes::receiver`), what it takes after its
//! arguments, in which order (`evidence_slots`), an `Option` a caller
//! gives `null` for (`nullable_params`), a rest parameter (`takes_rest`),
//! which trailing parameters a caller may leave out (`optional_from`), a
//! result whose `None` is `null` (`nullable_result`) and each parameter's
//! name (`parameter_name`). A declaration prints it, and says nothing a
//! function takes on its own (#549, #559, #560).

use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Ident;
use rustc_span::def_id::DefId;

use super::FnCx;
use super::classes::Receiver;
use super::traits::{Scope, Slot};

/// What a function's JS takes, in order, and gives.
pub(in crate::lower) struct JsSignature<'tcx> {
    /// What a JS caller gives it: its Rust parameters, but a `self` its JS
    /// takes as `this` or not at all (`classes::receiver`), then what it
    /// takes after them.
    pub(in crate::lower) params: Vec<JsParam<'tcx>>,
    /// What it gives.
    pub(in crate::lower) result: Form<'tcx>,
    /// Its own type parameters it's given a JS function of.
    pub(in crate::lower) callables: Vec<Callable<'tcx>>,
}

/// One of a function's own type parameters its JS is given a JS function
/// of, `F: Fn(&Item, usize) -> R` or an `impl Fn(..)`: a closure, a
/// function or a fn pointer, each a JS function (ADR 0099), by its index,
/// with what the function takes and gives (#661).
#[derive(Clone, Copy)]
pub(in crate::lower) struct Callable<'tcx> {
    pub(in crate::lower) index: u32,
    pub(in crate::lower) inputs: &'tcx [Ty<'tcx>],
    pub(in crate::lower) output: Ty<'tcx>,
}

/// One of what a function's JS takes.
pub(in crate::lower) struct JsParam<'tcx> {
    pub(in crate::lower) origin: Origin<'tcx>,
    /// Its name, as its body names it, a word JS keeps given apart, `new$`.
    pub(in crate::lower) name: String,
    pub(in crate::lower) form: Form<'tcx>,
}

/// What a parameter of a function's JS is of.
#[derive(Clone, Copy)]
pub(in crate::lower) enum Origin<'tcx> {
    /// Its Rust parameter, by its index in its signature.
    Rust(usize),
    /// What it takes after them (`evidence_slots`).
    Evidence(Slot<'tcx>),
}

/// What a JS caller gives, or is given, of a value: the TypeScript type it
/// is declared as is the representation owners' of `ty`'s parts (`&mut`'s
/// `MutRefForm::ts_type`, a `dyn`'s pair).
#[derive(Clone, Copy)]
pub(in crate::lower) enum Form<'tcx> {
    /// A value of `ty`, which a caller may leave out where `optional`, a
    /// trailing `Option`'s, whose `undefined` is `None` (ADR 0443).
    Value { ty: Ty<'tcx>, optional: bool },
    /// `T | null`, an `Option` of `inner`: a parameter marked
    /// `#[rust_js::nullable(..)]`, given `null` for `None`, never left out
    /// (ADR 0861, amended), and a nullable result (ADRs 0574, 0578).
    Nullable(Ty<'tcx>),
    /// `...args: T[]`, a slice marked `#[rust_js::rest]`, of type `ty` (ADR
    /// 0664).
    Rest(Ty<'tcx>),
    /// A bound's dictionary (ADR 0049), which a JS caller gives where it's
    /// declared, a trait's of the crate's (#550, ADR 1077, amended).
    Dictionary(ty::TraitRef<'tcx>),
    /// What only a Rust caller knows, a fact of a type parameter, `TSize`
    /// (ADR 0145): no JS caller can give it (#549, ADR 1077).
    Unknowable,
    /// What a JS caller leaves out, a type parameter's drop, `undefined` of
    /// one that drops nothing (ADRs 0098, 1077).
    LeftOut,
    /// Nothing, of a result: `()`'s, `void`.
    Void,
}

/// Where a function's parameters, or a function type's, may start being
/// left out: its trailing `Option`s, before a rest parameter `rest` too,
/// but one `nullable`, which is `T | null` and given (ADRs 0443, 0664,
/// 0861). The one rule of both, as a caller leaves a trailing `None` out
/// of each (ADR 0313, #544).
pub(in crate::lower) fn optional_from<'tcx>(
    tcx: TyCtxt<'tcx>,
    inputs: &[Ty<'tcx>],
    rest: Option<usize>,
    nullable: impl Fn(usize) -> bool,
) -> usize {
    let before = &inputs[..rest.unwrap_or(inputs.len())];
    before.len()
        - (before.iter().enumerate().rev())
            .take_while(|&(i, &ty)| is_option(tcx, ty) && !nullable(i))
            .count()
}

fn is_option<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option))
}

/// A parameter's name, `ident`, `function`'s of type `ty`, as its JS spells
/// it, a word JS keeps given apart, `new$`; its JS name's, without a `_`
/// of one unused (ADR 0678). Its `self`, named after its type, as its body
/// names it, `tick(counter: Counter)` (#507).
pub(in crate::lower) fn js_parameter_name<'tcx>(
    tcx: TyCtxt<'tcx>,
    function: DefId,
    ident: Ident,
    ty: Option<Ty<'tcx>>,
) -> String {
    let receiver = (ident.name == rustc_span::kw::SelfLower)
        .then(|| ty.and_then(|ty| super::analysis::receiver_name(tcx, ty)))
        .flatten();
    let name = receiver.unwrap_or_else(|| ident.name.to_string());
    let named = super::parameter_name(tcx, function, &name);
    crate::names::js_ident(&named)
}

impl<'tcx> JsSignature<'tcx> {
    /// The first of what `function`'s JS takes that a JS caller can't give,
    /// described: a fact of a type parameter, `` `T`'s size_of ``, first,
    /// then a bound's dictionary `declared` says isn't declared, `` `T`'s
    /// `Display` dictionary `` (#549, ADR 1077). `None` where it can give
    /// each.
    pub(in crate::lower) fn ungiven(
        &self,
        tcx: TyCtxt<'tcx>,
        function: DefId,
        declared: impl Fn(DefId) -> bool,
    ) -> Option<String> {
        let generics = tcx.generics_of(function);
        let fact = self.params.iter().find_map(|param| match (param.origin, param.form) {
            (Origin::Evidence(Slot::Fact(index, fact)), Form::Unknowable) => Some((index, fact)),
            _ => None,
        });
        if let Some((index, fact)) = fact {
            let of = match (index as usize) < generics.count() {
                true => format!("`{}`'s", generics.param_at(index as usize, tcx).name),
                false => "an associated type's".to_string(),
            };
            return Some(format!("{of} {}", fact.described()));
        }
        self.params.iter().find_map(|param| match param.form {
            Form::Dictionary(bound) if !declared(bound.def_id) => {
                let (of, name) = (bound.self_ty(), tcx.item_name(bound.def_id));
                Some(format!("`{of}`'s `{name}` dictionary"))
            }
            _ => None,
        })
    }
}

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// What `function`'s JS takes and gives, as a JS caller calls it.
    pub(in crate::lower) fn js_signature(&self, function: DefId) -> JsSignature<'tcx> {
        let tcx = self.tcx;
        let sig = (tcx.fn_sig(function).instantiate_identity())
            .skip_normalization()
            .skip_binder();
        let inputs = sig.inputs();
        let idents = tcx.fn_arg_idents(function);
        let rest = super::bindings::takes_rest(tcx, function).then(|| inputs.len() - 1);
        let nullable_params = super::bindings::nullable_params(tcx, function);
        let nullable = |i: usize| {
            (idents.get(i).copied().flatten()).is_some_and(|ident| nullable_params.contains(&ident.name))
                && is_option(tcx, inputs[i])
        };
        // What its JS takes after them, of a function a JS caller calls,
        // not a trait's method's, which its dictionary's type declares, nor a
        // binding's; after which none may be left out.
        let own = tcx.def_kind(function) == DefKind::Fn
            || (tcx.trait_of_assoc(function).is_none() && tcx.trait_item_of(function).is_none());
        let slots = match own && !super::bindings::is_binding(tcx, function) {
            true => self.evidence_slots(function, Scope::Whole),
            false => Vec::new(),
        };
        let given_after = slots.iter().any(|slot| !matches!(slot, Slot::Drop(_)));
        let optional_from = match given_after {
            true => inputs.len(),
            false => optional_from(tcx, inputs, rest, nullable),
        };
        let skipped = usize::from(super::classes::receiver(tcx, function) != Receiver::First && !inputs.is_empty());
        let mut params: Vec<JsParam<'tcx>> = (inputs.iter().enumerate().skip(skipped))
            .map(|(i, &ty)| {
                let name = match idents.get(i).copied().flatten() {
                    Some(ident) => js_parameter_name(tcx, function, ident, Some(ty)),
                    // A pattern, `TagProps { variant, .. }`: its props'.
                    None => "props".to_string(),
                };
                let form = match (rest == Some(i), nullable(i)) {
                    (true, _) => Form::Rest(ty),
                    (false, true) => match ty.kind() {
                        ty::Adt(_, args) => Form::Nullable(args.type_at(0)),
                        _ => unreachable!("a nullable parameter is an `Option`"),
                    },
                    (false, false) => Form::Value {
                        ty,
                        optional: i >= optional_from,
                    },
                };
                JsParam {
                    origin: Origin::Rust(i),
                    name,
                    form,
                }
            })
            .collect();
        let mut taken: std::collections::HashSet<String> = (idents.iter().enumerate())
            .filter_map(|(i, ident)| {
                let ident = ident.filter(|ident| ident.name != rustc_span::kw::Underscore)?;
                Some(js_parameter_name(tcx, function, ident, inputs.get(i).copied()))
            })
            .collect();
        for slot in slots {
            let (base, form) = match slot {
                // Its caller's location, which a JS caller gives as a Rust
                // caller does, where it's called from (ADR 0427, #560).
                Slot::Location => (
                    "location".to_string(),
                    Form::Value {
                        ty: tcx.caller_location_ty(),
                        optional: false,
                    },
                ),
                // A const parameter's value, `N`, a number a JS caller gives
                // as it's typed (ADR 0107, #560).
                Slot::Const(index) => {
                    let param = tcx.generics_of(function).param_at(index as usize, tcx);
                    let ty = tcx.type_of(param.def_id).instantiate_identity().skip_normalization();
                    (param.name.to_string(), Form::Value { ty, optional: false })
                }
                Slot::Bound(bound) => (super::traits::bound_word(tcx, bound), Form::Dictionary(bound)),
                Slot::Fact(..) => (String::new(), Form::Unknowable),
                Slot::Drop(_) => (String::new(), Form::LeftOut),
            };
            let name = match base.is_empty() {
                true => base,
                false => crate::names::fresh_in(&mut taken, &base),
            };
            params.push(JsParam {
                origin: Origin::Evidence(slot),
                name,
                form,
            });
        }
        let output = sig.output();
        let result = match (output.is_unit(), super::bindings::nullable_result(tcx, function)) {
            (true, _) => Form::Void,
            (false, true) if is_option(tcx, output) => match output.kind() {
                ty::Adt(_, args) => Form::Nullable(args.type_at(0)),
                _ => unreachable!("an `Option` is an ADT"),
            },
            (false, _) => Form::Value {
                ty: output,
                optional: false,
            },
        };
        JsSignature {
            params,
            result,
            callables: self.callables(function),
        }
    }

    /// `function`'s own type parameters bound by a function trait, `F:
    /// Fn(&Item) -> R`, a value of which is a JS function (`is_callable`),
    /// each with what its bound says it takes and gives, its `Output`'s or
    /// `()` (#661). Not its impl's, which its type's declaration takes as
    /// its own.
    fn callables(&self, function: DefId) -> Vec<Callable<'tcx>> {
        let tcx = self.tcx;
        let parent = tcx.generics_of(function).parent_count as u32;
        let clauses = tcx.clauses_of(function).clauses;
        let mut callables: Vec<Callable<'tcx>> = Vec::new();
        for (clause, _) in clauses.iter() {
            let Some(bound) = clause.as_trait_clause().map(|b| b.skip_binder()) else {
                continue;
            };
            let ty::Param(param) = bound.self_ty().kind() else {
                continue;
            };
            if param.index < parent
                || callables.iter().any(|callable| callable.index == param.index)
                || tcx.fn_trait_kind_from_def_id(bound.def_id()).is_none()
                || !self.is_callable(bound.self_ty())
            {
                continue;
            }
            let ty::Tuple(inputs) = bound.trait_ref.args.type_at(1).kind() else {
                continue;
            };
            let output = (clauses.iter())
                .filter_map(|(clause, _)| clause.as_projection_clause())
                .map(|p| p.skip_binder())
                .find(|p| matches!(p.self_ty().kind(), ty::Param(q) if q.index == param.index))
                .and_then(|p| p.term.as_type())
                .unwrap_or(tcx.types.unit);
            callables.push(Callable {
                index: param.index,
                inputs: inputs.as_slice(),
                output,
            });
        }
        callables
    }
}
