//! What a function's JS takes and gives, as a JS caller calls it: its
//! `JsSignature`, lowering's answer (ADR 1091). Each part is its owner's:
//! what its `self` is (`classes::receiver`), what it takes after its
//! arguments, in which order (`evidence_slots`), an `Option` a caller
//! gives `null` for (`nullable_params`), a rest parameter (`takes_rest`),
//! which trailing parameters a caller may leave out (`optional_from`), a
//! result whose `None` is `null` (`nullable_result`) and each parameter's
//! name (`parameter_name`). A declaration prints it, and says nothing a
//! function takes on its own (#549, #559, #560).

use rust_js_program::collections::{IndexMap, IndexSet};
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::mir;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Ident;
use rustc_span::def_id::DefId;

use super::FnCx;
use super::classes::Receiver;
use super::traits::{Scope, Slot};

/// What a function's JS takes, in order, and gives.
pub struct JsSignature<'tcx> {
    /// What a JS caller gives it: its Rust parameters, but a `self` its JS
    /// takes as `this` or not at all (`classes::receiver`), then what it
    /// takes after them.
    pub(crate) params: Vec<JsParam<'tcx>>,
    /// What it gives.
    pub(crate) result: Form<'tcx>,
    /// Its own type parameters it's given a JS function of.
    pub(crate) callables: Vec<Callable<'tcx>>,
}

/// One of a function's own type parameters its JS is given a JS function
/// of, `F: Fn(&Item, usize) -> R` or an `impl Fn(..)`: a closure, a
/// function or a fn pointer, each a JS function (ADR 0099), by its index,
/// with what the function takes and gives (#661).
#[derive(Clone, Copy)]
pub(crate) struct Callable<'tcx> {
    pub(crate) index: u32,
    pub(crate) inputs: &'tcx [Ty<'tcx>],
    pub(crate) output: Ty<'tcx>,
}

/// One of what a function's JS takes.
pub(crate) struct JsParam<'tcx> {
    pub(crate) origin: Origin<'tcx>,
    /// Its name, as its body names it, a word JS keeps given apart, `new$`.
    pub(crate) name: String,
    pub(crate) form: Form<'tcx>,
}

/// What a parameter of a function's JS is of.
#[derive(Clone, Copy)]
pub(crate) enum Origin<'tcx> {
    /// Its Rust parameter, by its index in its signature.
    Rust(usize),
    /// What it takes after them (`evidence_slots`).
    Evidence(Slot<'tcx>),
}

/// What a JS caller gives, or is given, of a value: the TypeScript type it
/// is declared as is the representation owners' of `ty`'s parts (`&mut`'s
/// `MutRefForm::ts_type`, a `dyn`'s pair).
#[derive(Clone, Copy)]
pub(crate) enum Form<'tcx> {
    /// A value of `ty`, which a caller may leave out where `optional`, a
    /// trailing `Option`'s, whose `undefined` is `None` (ADR 0443).
    Value { ty: Ty<'tcx>, optional: bool },
    /// `T | null`, an `Option` of `inner`: a parameter marked
    /// `#[rust_js::nullable(..)]`, given `null` for `None`, never left out
    /// (ADR 0861, amended), and a nullable result (ADRs 0574, 0578).
    Nullable(Ty<'tcx>),
    /// `T | undefined`, an `Option` of `inner` whose `None` is `undefined`
    /// alone, never `null`: a parameter marked
    /// `#[rust_js::none_undefined(..)]`, which a caller may leave out where
    /// `optional`, upstream's `cancelable?: Cancelable` (ADR 1161, amended).
    Undefinable { inner: Ty<'tcx>, optional: bool },
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
pub(crate) fn optional_from<'tcx>(
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
pub(crate) fn js_parameter_name<'tcx>(
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
    rust_js_program::names::js_ident(&named)
}

/// Each of `function`'s parameters' names as its own JS writes them
/// (`js_parameter_name`), which an arrow of it, a function value's, a
/// dictionary's entry, takes too, `(n) => bracketed(n, IsI32)` (#1328, the
/// user's decision): `None` of one that names none, `_` or a pattern, and
/// of a `self` whose type's name is another parameter's, `String::
/// push_str`'s `string` beside its `string`, or of std's that its type
/// names none of, `u8::is_ascii_lowercase`'s, no program's `self`: each
/// the caller names, by its type where it can, `(n) => ..`.
pub fn own_parameter_names(tcx: TyCtxt<'_>, function: DefId) -> Vec<Option<String>> {
    let inputs = super::traits::erased_sig(tcx, function).inputs().to_vec();
    let idents = tcx.fn_arg_idents(function);
    let names: Vec<Option<String>> = (inputs.iter().enumerate())
        .map(|(i, &ty)| {
            let ident = (idents.get(i).copied().flatten()).filter(|ident| ident.name != rustc_span::kw::Underscore)?;
            Some(js_parameter_name(tcx, function, ident, Some(ty)))
        })
        .collect();
    let std = super::recognition::is_std_crate(tcx, function);
    let receiver =
        |i: usize| (idents.get(i).copied().flatten()).is_some_and(|ident| ident.name == rustc_span::kw::SelfLower);
    (names.iter().enumerate())
        .map(|(i, name)| {
            let taken = |name: &String| {
                names
                    .iter()
                    .enumerate()
                    .any(|(j, other)| j != i && other.as_ref() == Some(name))
            };
            name.clone()
                .filter(|name| !(receiver(i) && ((std && name == "self") || taken(name))))
        })
        .collect()
}

/// What an arrow of `function` names each parameter its JS takes (#1328):
/// `own_parameter_names`', but its `Formatter`'s, `formatter`, which none
/// of its JS takes (ADR 0054), and a writer's `options` last, where it
/// takes them (ADRs 0058, 0137).
pub(crate) fn taken_parameter_names(
    tcx: TyCtxt<'_>,
    function: DefId,
    formatter: Option<usize>,
    options: bool,
) -> Vec<Option<String>> {
    let mut names: Vec<Option<String>> = (own_parameter_names(tcx, function).into_iter().enumerate())
        .filter(|&(i, _)| Some(i) != formatter)
        .map(|(_, name)| name)
        .collect();
    names.extend((options && formatter.is_some()).then(|| Some("options".to_string())));
    names
}

impl<'tcx> JsSignature<'tcx> {
    /// The first of what `function`'s JS takes that a JS caller can't give,
    /// described: a fact of a type parameter, `` `T`'s size_of ``, first,
    /// then a bound's dictionary `declared` says isn't declared, `` `T`'s
    /// `Display` dictionary `` (#549, ADR 1077). `None` where it can give
    /// each.
    pub(crate) fn ungiven(
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
    /// Of `body`, each parameter whose `None` is `undefined` alone
    /// (`Form::Undefinable`, `#[rust_js::none_undefined(..)]`) that its
    /// first act gives a literal default, `let width = width.unwrap_or(4.0);`,
    /// and that literal: its JS parameter's own default, `width = 4`, as
    /// upstream writes one, applied by JS where the argument is `undefined`
    /// or left out (#2077, #2081, ADR 1161, amended). The entry's calls
    /// before any other effect, each an `unwrap_or` of one such parameter,
    /// or of a temporary copy of it, with a constant of a number or a
    /// `bool`, its JS a literal; a
    /// parameter read anywhere else isn't one, as its `None` would be the
    /// default there. The one answer both its parameter's pattern and the
    /// call's lowering ask (`mir_param`, `defaulted_present`).
    pub fn literal_defaults(&self, body: &mir::Body<'tcx>) -> IndexMap<mir::Local, mir::ConstOperand<'tcx>> {
        let mut found = IndexMap::default();
        let marked = super::bindings::undefined_params(self.tcx, body.source.def_id());
        if marked.is_empty() {
            return found;
        }
        let params: IndexSet<mir::Local> = (body.var_debug_info.iter())
            .filter_map(|info| match info.value {
                mir::VarDebugInfoContents::Place(place)
                    if place.projection.is_empty()
                        && (1..=body.arg_count).contains(&place.local.as_usize())
                        && marked.contains(&info.name) =>
                {
                    Some(place.local)
                }
                _ => None,
            })
            .collect();
        // Each temporary that copies one of them before its `unwrap_or`, to it.
        let mut copies: IndexMap<mir::Local, mir::Local> = IndexMap::default();
        let mut block = mir::START_BLOCK;
        loop {
            let data = &body.basic_blocks[block];
            for statement in &data.statements {
                match &statement.kind {
                    // What does nothing: a `let`'s markers of what it binds.
                    mir::StatementKind::StorageLive(_)
                    | mir::StatementKind::StorageDead(_)
                    | mir::StatementKind::FakeRead(_)
                    | mir::StatementKind::PlaceMention(_)
                    | mir::StatementKind::AscribeUserType(..)
                    | mir::StatementKind::Coverage(_)
                    | mir::StatementKind::Nop => {}
                    mir::StatementKind::Assign(assign)
                        if let (dest, mir::Rvalue::Use(mir::Operand::Copy(of) | mir::Operand::Move(of), _)) =
                            &**assign
                            && dest.projection.is_empty()
                            && of.projection.is_empty()
                            && dest.local.as_usize() > body.arg_count
                            && params.contains(&of.local) =>
                    {
                        copies.insert(dest.local, of.local);
                    }
                    _ => return found,
                }
            }
            let mir::TerminatorKind::Call {
                func,
                args,
                destination,
                target: Some(next),
                ..
            } = &data.terminator().kind
            else {
                return found;
            };
            let Some((id, generic)) = super::fn_def(func.ty(&body.local_decls, self.tcx)) else {
                return found;
            };
            // A `map` of one that's the option itself in JS,
            // `cancelable.map(js::boolean)` (`maps_to_itself`): what it
            // gives is the parameter as a copy is.
            if let [option, f] = &args[..]
                && let mir::Operand::Copy(given) | mir::Operand::Move(given) = &option.node
                && let Some(&param) = copies.get(&given.local).or(params.get(&given.local))
                && destination.projection.is_empty()
                && self.recognition().classify(id, generic) == Some(super::recognition::Std::OptionMap)
                && let (Some(from), Some(to)) = (
                    self.option_of(option.node.ty(body, self.tcx)),
                    self.option_of(destination.ty(body, self.tcx).ty),
                )
                && self.maps_to_itself(f.node.ty(body, self.tcx), (from, to))
            {
                copies.insert(destination.local, param);
                block = *next;
                continue;
            }
            let [option, default] = &args[..] else {
                return found;
            };
            let (mir::Operand::Copy(given) | mir::Operand::Move(given), mir::Operand::Constant(literal)) =
                (&option.node, &default.node)
            else {
                return found;
            };
            let param = copies.get(&given.local).copied().unwrap_or(given.local);
            let literal_ty = literal.ty();
            if self.recognition().classify(id, generic) != Some(super::recognition::Std::UnwrapOr)
                || !given.projection.is_empty()
                || !params.contains(&param)
                || found.contains_key(&param)
                || !(literal_ty.is_bool() || literal_ty.is_integral() || literal_ty.is_floating_point())
                || !finite(literal)
                || uses(body, param) != 1
            {
                return found;
            }
            found.insert(param, **literal);
            block = *next;
        }
    }

    /// Whether `local` of `body` is a parameter a literal default made a
    /// value (`literal_defaults`), read by its `unwrap_or` alone, so it's
    /// present there, its `unwrap_or` the value (`proved_present`, #2081).
    pub fn defaulted_present(&self, body: &mir::Body<'tcx>, local: mir::Local) -> bool {
        self.literal_defaults(body).contains_key(&local)
    }

    /// How many parameters an impl's method's JS function declares, as
    /// `mir_signature` and `lower_fn_mir` write them: one for each of its
    /// Rust inputs but a `self` its body takes as `this` or not at all
    /// (`classes::body_receiver`, `receiver_as_this`'s) and a writer's
    /// `Formatter`, that `Formatter`'s `options` where the crate gives
    /// them, then one for each of its `evidence_slots`. `None` of one whose last gathers what it's given
    /// past the rest, `#[rust_js::rest]`, C-variadic or rest arguments as a
    /// value, and of any other function, a free one's unnamed last
    /// parameters left out (ADR 0678): what a dictionary's entry may pass a
    /// caller's arguments on to as they are (#801).
    pub(crate) fn declared_arity(&self, def_id: DefId) -> Option<usize> {
        if self.tcx.def_kind(def_id) != DefKind::AssocFn
            || super::recognition::is_c_variadic(self.tcx, def_id)
            || self.takes_rest(def_id)
        {
            return None;
        }
        let inputs = self.tcx.fn_sig(def_id).skip_binder().inputs().skip_binder();
        if inputs
            .last()
            .is_some_and(|&last| super::bindings::is_rest_value(self.tcx, last))
        {
            return None;
        }
        let receiver = !inputs.is_empty() && super::classes::body_receiver(self.tcx, def_id) != Receiver::First;
        let formatter = self.formatter_param(def_id).is_some();
        let options = formatter && self.compiled_writer_takes_options(def_id);
        let evidence = self.evidence_slots(def_id, Scope::Whole).len();
        Some(inputs.len() - usize::from(receiver) - usize::from(formatter) + usize::from(options) + evidence)
    }

    /// What `lower_fn_mir` wrote of an impl's method, its `count` of
    /// parameters, is what `declared_arity` says, which a dictionary's entry
    /// passes a caller's arguments on to: each compile in a debug build
    /// holds the two equal (#801).
    pub fn holds_declared_arity(&self, def_id: DefId, count: usize) {
        if let Some(declared) = self.declared_arity(def_id) {
            debug_assert_eq!(
                declared,
                count,
                "`declared_arity` of {} isn't its parameters' count",
                self.tcx.def_path_str(def_id)
            );
        }
    }

    /// What `function`'s JS takes and gives, as a JS caller calls it.
    pub fn js_signature(&self, function: DefId) -> JsSignature<'tcx> {
        let tcx = self.tcx;
        let sig = (tcx.fn_sig(function).instantiate_identity())
            .skip_normalization()
            .skip_binder();
        let inputs = sig.inputs();
        let idents = tcx.fn_arg_idents(function);
        let rest = super::bindings::takes_rest(tcx, function).then(|| inputs.len() - 1);
        let null_params = super::representation::null_params(tcx, function);
        // A present one's a caller gives, `T | undefined`, no more left out
        // than a nullable one (ADR 1161, #1845).
        let present = super::bindings::present_params(tcx, function);
        let present_at = |i: usize| {
            idents
                .get(i)
                .copied()
                .flatten()
                .is_some_and(|ident| present.contains(&ident.name))
        };
        let nullable = |i: usize| null_params.contains(&i) && is_option(tcx, inputs[i]);
        let undefined = super::bindings::undefined_params(tcx, function);
        let undefinable = |i: usize| {
            is_option(tcx, inputs[i])
                && !nullable(i)
                && (idents.get(i).copied().flatten()).is_some_and(|ident| undefined.contains(&ident.name))
        };
        let required = |i: usize| (null_params.contains(&i) || present_at(i)) && is_option(tcx, inputs[i]);
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
            false => optional_from(tcx, inputs, rest, required),
        };
        let skipped = usize::from(super::classes::receiver(tcx, function) != Receiver::First && !inputs.is_empty());
        let mut params: Vec<JsParam<'tcx>> = (inputs.iter().enumerate().skip(skipped))
            .map(|(i, &ty)| {
                let name = match idents.get(i).copied().flatten() {
                    Some(ident) => js_parameter_name(tcx, function, ident, Some(ty)),
                    // A pattern, `TagProps { variant, .. }`: its props'.
                    None => "props".to_string(),
                };
                let optional = i >= optional_from;
                let form = match (rest == Some(i), nullable(i)) {
                    (true, _) => Form::Rest(ty),
                    (false, true) => match ty.kind() {
                        ty::Adt(_, args) => Form::Nullable(args.type_at(0)),
                        _ => unreachable!("a nullable parameter is an `Option`"),
                    },
                    (false, false) if undefinable(i) => match ty.kind() {
                        ty::Adt(_, args) => Form::Undefinable {
                            inner: args.type_at(0),
                            optional,
                        },
                        _ => unreachable!("an undefinable parameter is an `Option`"),
                    },
                    (false, false) => Form::Value { ty, optional },
                };
                JsParam {
                    origin: Origin::Rust(i),
                    name,
                    form,
                }
            })
            .collect();
        let mut taken: IndexSet<String> = (idents.iter().enumerate())
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
                // How std collects a `Vec` of it, `IVecCollect`, which a JS
                // caller leaves out: what it gives is no Rust iterator, which
                // `undefined` collects by `next`, as JS steps it (#1239).
                Slot::Fact(_, crate::recognition::TypeFact::VecCollect) => (String::new(), Form::LeftOut),
                // Whether a counted `Rc` of it is a class's instance,
                // `TInstance`, which a JS caller leaves out: a box, which the
                // `Rc`'s readers tell apart from an instance as it runs (#1423).
                Slot::Fact(_, crate::recognition::TypeFact::Instance) => (String::new(), Form::LeftOut),
                Slot::Fact(..) => (String::new(), Form::Unknowable),
                Slot::Drop(_) => (String::new(), Form::LeftOut),
            };
            let name = match base.is_empty() {
                true => base,
                false => rust_js_program::names::fresh_in(&mut taken, &base),
            };
            params.push(JsParam {
                origin: Origin::Evidence(slot),
                name,
                form,
            });
        }
        let output = sig.output();
        let null_result =
            super::representation::none_form(tcx, super::representation::NonePlace::FnResult(function)).stores_null();
        let result = match (output.is_unit(), null_result) {
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
    /// `()` (#661). Its own clauses alone: an impl's `F: Fn()` is its
    /// impl's, which its type's declaration takes as its parameter.
    fn callables(&self, function: DefId) -> Vec<Callable<'tcx>> {
        let tcx = self.tcx;
        let clauses = tcx.clauses_of(function).clauses;
        let mut callables: Vec<Callable<'tcx>> = Vec::new();
        for (clause, _) in clauses.iter() {
            let Some(bound) = clause.as_trait_clause().map(|b| b.skip_binder()) else {
                continue;
            };
            let ty::Param(param) = bound.self_ty().kind() else {
                continue;
            };
            if callables.iter().any(|callable| callable.index == param.index)
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

/// Whether `literal`, a float, is finite, its JS a number literal, not
/// `Infinity` or `NaN`; any other constant is.
fn finite(literal: &mir::ConstOperand<'_>) -> bool {
    let mir::Const::Val(mir::ConstValue::Scalar(mir::interpret::Scalar::Int(bits)), ty) = literal.const_ else {
        return !literal.ty().is_floating_point();
    };
    match ty.kind() {
        ty::Float(ty::FloatTy::F64) => f64::from_bits(bits.to_bits(bits.size()) as u64).is_finite(),
        ty::Float(ty::FloatTy::F32) => f32::from_bits(bits.to_bits(bits.size()) as u32).is_finite(),
        ty::Float(_) => false,
        _ => true,
    }
}

/// How many times `body` reads or writes `local`, but its storage's
/// markers and its debug info.
fn uses(body: &mir::Body<'_>, local: mir::Local) -> usize {
    struct Uses(mir::Local, usize);
    impl<'tcx> mir::visit::Visitor<'tcx> for Uses {
        fn visit_local(&mut self, local: mir::Local, context: mir::visit::PlaceContext, _: mir::Location) {
            if local == self.0 && !matches!(context, mir::visit::PlaceContext::NonUse(_)) {
                self.1 += 1;
            }
        }
    }
    let mut counted = Uses(local, 0);
    mir::visit::Visitor::visit_body(&mut counted, body);
    counted.1
}
