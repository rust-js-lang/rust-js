//! A type's boundary contract: what a value of it is to JS, as the
//! representation owners say, decided once for each type (#2403, ADR 1190).
//! The declarations print it as TypeScript (`Declarations::ts`), and the
//! checks of what JS gives Rust compile it, so the two can't disagree. It
//! classifies a type alone; what's printed of each kind, and what's
//! imported or declared on the way, is the printer's.

use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, GenericArgsRef, Ty, TyCtxt};
use rustc_span::def_id::DefId;

use super::written::{Written, written_types};
use crate::bindings::{UnitValue, is_rest, is_untagged};
use crate::iterators::boundary::BoundaryIterator;
use crate::recognition::{StdItem, is_std_def};
use crate::representation::Num;

/// What the contract of a type is asked from: the analyses that decide what
/// a value is in JS where its type alone doesn't.
pub(crate) struct Facts<'a, 'tcx> {
    pub(crate) tcx: TyCtxt<'tcx>,
    /// The `Rc`s counted, `{ value, strong, weak }`: every other, and its
    /// `Weak`, is the value it points at (ADRs 0320, 0843).
    pub(crate) counted: &'a crate::analysis::Counted<'tcx>,
    /// The `RefCell`s a field holds as their value (ADR 0362).
    pub(crate) ref_cells: &'a crate::drops::RefCells<'tcx>,
    /// What a library's contract says its declarations export (ADR 0210).
    pub(crate) foreign: &'a crate::library::Foreign<'a, 'tcx>,
}

/// What a value of a type is to JS, by the owner that decides it.
pub(crate) enum Contract<'tcx> {
    /// A number of its kind: a `number`, or a 64-bit or wider one's
    /// `bigint` (`Num`).
    Num(Num),
    /// A `boolean`.
    Bool,
    /// A `char`: a `string` of one character.
    Char,
    /// A `str`, a `String`, or a panic's `Location`, its text (ADR 0427).
    Str,
    /// A `&mut` to its pointee: a cell, a box or a handle of a value JS
    /// can't change in place, or the object (`MutRefForm`, ADR 1086).
    MutRef(Ty<'tcx>),
    /// A shared borrow: what it refers to, lent to be read (ADR 1189).
    Shared(Ty<'tcx>),
    /// What a function that never returns returns (ADR 0386).
    Never,
    /// `()`: `undefined`.
    Unit,
    /// A tuple of these: an array of its items.
    Tuple(&'tcx ty::List<Ty<'tcx>>),
    /// An array or a slice of these.
    Items(Ty<'tcx>),
    /// A type parameter.
    Param(ty::ParamTy),
    /// A fn pointer: a JS function.
    FnPtr,
    /// A `dyn Fn` of these: a JS function.
    DynFn { inputs: &'tcx [Ty<'tcx>], output: Ty<'tcx> },
    /// A `dyn` of a trait that says what it is to TypeScript (ADR 0242).
    DynWritten(DefId, Written),
    /// A type map's associated type, of its reason (ADR 1164).
    TypeMapProjection(DefId, GenericArgsRef<'tcx>),
    /// A binding trait's associated type that says what it is (#1817).
    WrittenProjection(DefId, GenericArgsRef<'tcx>),
    /// An `impl Future`'s `Promise` of what it resolves to (ADR 0029).
    Future(Ty<'tcx>),
    /// An `impl Iterator` a function gives, the JS iterator (#1999).
    Iterator(BoundaryIterator<'tcx>),
    /// An `impl Fn(..)` a function gives: the JS function it is.
    OpaqueFn(Vec<Ty<'tcx>>, Ty<'tcx>),
    /// Any other `dyn`: its pair, its value and its impl's dictionary, or
    /// `any` (#620).
    Dyn,
    /// A closure's own type, which no signature names.
    Closure,
    /// An `Option` of its payload, `None` JS's `null` or `undefined`
    /// (ADR 0030).
    Option(Ty<'tcx>),
    /// One of the js crate's literal types, the literal it is (ADR 1164).
    JsLiteral(UnitValue),
    /// A `Vec` of these: an array.
    Vec(Ty<'tcx>),
    /// A `HashMap` or a `BTreeMap`: a `Map` (ADRs 0059, 0121, 0876).
    Map(Ty<'tcx>, Ty<'tcx>),
    /// A `HashSet` or a `BTreeSet`: a `Set`.
    Set(Ty<'tcx>),
    /// What a pointer is: a `Box`'s, an uncounted `Rc`'s or a counted one
    /// of a class (ADRs 0023, 0843, #1228), a `RefCell` held as its value
    /// (ADR 0863): what it points at.
    Pointee(Ty<'tcx>),
    /// An uncounted `Weak`: what it points at, or `null`.
    Weak(Ty<'tcx>),
    /// A `Cell`, or a `RefCell` that isn't what it holds: `{ value }`
    /// (ADRs 0023, 0288, 0328, 0863).
    Held(Ty<'tcx>),
    /// Rest props, `Record<string, unknown>`.
    Rest,
    /// A typed JS class, `Class<fn(A) -> T>`, of its constructor's
    /// signature (ADR 1009).
    JsClass(Ty<'tcx>),
    /// One of std's opaque to TypeScript, as `@rust-js/std` declares it
    /// (ADR 1149).
    StdOpaque(&'static super::std_types::Opaque, GenericArgsRef<'tcx>),
    /// A binding's type, as it says it's typed.
    Written(DefId, Written, GenericArgsRef<'tcx>),
    /// A `Cow` or a `Result`: its variants by their `TAG` (ADR 0033, #1835).
    Tagged(DefId, GenericArgsRef<'tcx>),
    /// Another crate's untagged enum, the union of its payloads (ADR 0225).
    ForeignUntagged(DefId, GenericArgsRef<'tcx>),
    /// A module's namespace (ADR 1037).
    Namespace(DefId),
    /// A library's type its declarations export (ADR 0210, amended).
    Library(DefId, GenericArgsRef<'tcx>),
    /// One of the crate's own types.
    Local(DefId, GenericArgsRef<'tcx>),
    /// What has no type of its own to JS: `any`.
    Any,
}

impl<'tcx> Contract<'tcx> {
    /// The contract of `ty`. Each kind is asked of its owner, first to
    /// last as each decides.
    pub(crate) fn of(facts: &Facts<'_, 'tcx>, ty: Ty<'tcx>) -> Contract<'tcx> {
        let tcx = facts.tcx;
        if let Some(num) = Num::of(ty) {
            return Contract::Num(num);
        }
        match ty.kind() {
            ty::Bool => Contract::Bool,
            ty::Char => Contract::Char,
            ty::Str => Contract::Str,
            ty::Ref(_, inner, ty::Mutability::Mut) => Contract::MutRef(*inner),
            ty::Ref(_, inner, _) => Contract::Shared(*inner),
            ty::Never => Contract::Never,
            ty::Tuple(items) if items.is_empty() => Contract::Unit,
            ty::Tuple(items) => Contract::Tuple(items),
            ty::Array(item, _) | ty::Slice(item) => Contract::Items(*item),
            ty::Param(param) => Contract::Param(*param),
            ty::FnPtr(..) => Contract::FnPtr,
            ty::Dynamic(traits, ..)
                if traits
                    .principal_def_id()
                    .is_some_and(|t| tcx.fn_trait_kind_from_def_id(t).is_some()) =>
            {
                let inputs = match traits.principal().map(|p| p.skip_binder().args.type_at(0).kind()) {
                    Some(ty::Tuple(inputs)) => inputs.as_slice(),
                    _ => &[],
                };
                let output = (traits.projection_bounds())
                    .find_map(|output| output.skip_binder().term.as_type())
                    .unwrap_or(tcx.types.unit);
                Contract::DynFn { inputs, output }
            }
            ty::Dynamic(traits, ..)
                if let Some(t) = traits.principal_def_id()
                    && let Some(declared) = written_types(tcx, t) =>
            {
                Contract::DynWritten(t, declared)
            }
            ty::Alias(
                _,
                alias @ ty::AliasTy {
                    kind: ty::Projection { def_id },
                    ..
                },
            ) if super::type_maps::of_a_type_map(tcx, *def_id) => Contract::TypeMapProjection(*def_id, alias.args),
            ty::Alias(
                _,
                alias @ ty::AliasTy {
                    kind: ty::Projection { def_id },
                    ..
                },
            ) if written_types(tcx, *def_id).is_some() => Contract::WrittenProjection(*def_id, alias.args),
            ty::Alias(
                _,
                alias @ ty::AliasTy {
                    kind: ty::Opaque { .. },
                    ..
                },
            ) if let Some(output) = super::parts::opaque_future_output(tcx, *alias) => Contract::Future(output),
            ty::Alias(
                _,
                alias @ ty::AliasTy {
                    kind: ty::Opaque { .. },
                    ..
                },
            ) if let Some(boundary) = crate::iterators::boundary::of_opaque(tcx, *alias) => {
                Contract::Iterator(boundary)
            }
            ty::Alias(
                _,
                alias @ ty::AliasTy {
                    kind: ty::Opaque { .. },
                    ..
                },
            ) if let Some((inputs, output)) = super::parts::opaque_fn_signature(tcx, *alias) => {
                Contract::OpaqueFn(inputs, output)
            }
            ty::Dynamic(..) => Contract::Dyn,
            ty::Closure(..) => Contract::Closure,
            ty::Adt(adt, args) => Self::of_adt(facts, ty, *adt, args),
            _ => Contract::Any,
        }
    }

    /// The contract of a struct, an enum or a union, `ty`.
    fn of_adt(
        facts: &Facts<'_, 'tcx>,
        ty: Ty<'tcx>,
        adt: ty::AdtDef<'tcx>,
        args: GenericArgsRef<'tcx>,
    ) -> Contract<'tcx> {
        let tcx = facts.tcx;
        let did = adt.did();
        if tcx.is_lang_item(did, LangItem::Option) {
            return Contract::Option(args.type_at(0));
        }
        if tcx.is_lang_item(did, LangItem::String) {
            return Contract::Str;
        }
        if crate::recognition::is_js_literal_type(tcx, did)
            && let Some(value) = super::named_unit(tcx, did)
        {
            return Contract::JsLiteral(value);
        }
        if tcx.is_lang_item(did, LangItem::PanicLocation) {
            return Contract::Str;
        }
        if is_std_def(tcx, did, StdItem::Vec) {
            return Contract::Vec(args.type_at(0));
        }
        if [StdItem::HashMap, StdItem::BTreeMap]
            .iter()
            .any(|&map| is_std_def(tcx, did, map))
        {
            return Contract::Map(args.type_at(0), args.type_at(1));
        }
        if [StdItem::HashSet, StdItem::BTreeSet]
            .iter()
            .any(|&set| is_std_def(tcx, did, set))
        {
            return Contract::Set(args.type_at(0));
        }
        if ty.is_box() {
            return Contract::Pointee(args.type_at(0));
        }
        if let Some(pointee) = crate::recognition::rc_pointee(tcx, ty)
            && (!facts.counted.counts(tcx, pointee) || crate::std_types::rc::counted_as_instance(tcx, pointee))
        {
            return Contract::Pointee(pointee);
        }
        if let Some(pointee) = crate::recognition::weak_pointee(tcx, ty)
            && !facts.counted.counts(tcx, pointee)
        {
            return Contract::Weak(pointee);
        }
        if facts.ref_cells.is_value_cell(tcx, ty) {
            return Contract::Pointee(args.type_at(0));
        }
        if is_std_def(tcx, did, StdItem::Cell) || is_std_def(tcx, did, StdItem::RefCell) {
            return Contract::Held(args.type_at(0));
        }
        if is_rest(tcx, ty) {
            return Contract::Rest;
        }
        if crate::recognition::is_js_class(tcx, did)
            && let Some(signature) = args.types().next()
            && let ty::FnPtr(..) = signature.kind()
        {
            return Contract::JsClass(signature);
        }
        if let Some(opaque) = super::std_types::opaque(tcx, did) {
            return Contract::StdOpaque(opaque, args);
        }
        if let Some(declared) = written_types(tcx, did) {
            return Contract::Written(did, declared, args);
        }
        if crate::recognition::is_cow_type(tcx, ty) || is_std_def(tcx, did, StdItem::Result) {
            return Contract::Tagged(did, args);
        }
        if !did.is_local() && adt.is_enum() && is_untagged(tcx, did) {
            return Contract::ForeignUntagged(did, args);
        }
        if crate::classes::is_namespace(tcx, did) {
            return Contract::Namespace(did);
        }
        if facts.foreign.declared_type(did).is_some() {
            return Contract::Library(did, args);
        }
        if !did.is_local() {
            return Contract::Any;
        }
        Contract::Local(did, args)
    }
}
