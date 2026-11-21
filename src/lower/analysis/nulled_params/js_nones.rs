//! Which of the crate's `Option`s may hold a `None` JS gave, `null`, where
//! Rust's one is `undefined` (#1840, #1843; ADR 0030, amended). An `Option`
//! JS gives is declared `T | null | undefined`, and `is_none` is true of
//! either; inside Rust's code the two read alike, but where a `None` leaves
//! for JS as `undefined` alone, a native optional argument, a typed
//! caller's `=== undefined`, JS tells them apart. Each such exit asks
//! `JsNones::may_hold_null` of what it gives, and gives `value ?? undefined`
//! where it may (`mir::nulled`'s `none_trusted`).
//!
//! A flow-insensitive taint over every body's MIR: each local, each field
//! of a struct, a closure's or a coroutine's capture, and each static and
//! thread-local key is a node, of a type that may hold an `Option`. A move,
//! a copy, an aggregate's operand, a field's read and a call of a body the
//! crate has, its arguments to its parameters and its return to its
//! result, flow one way, and both ways where the value shares what it
//! points at, a reference's or an `Rc`'s, which a write through either
//! changes. A call of std's gives its result what each argument holds, and
//! each argument that points at something what the others hold; a closure
//! or a function it's given, what each argument holds, and its result what
//! that one returns. What isn't seen is a source: a parameter of what JS or
//! a consumer calls (`js_facing`, `library::reachable`), a nullable one,
//! one of a closure or a function given where its caller isn't seen (to a
//! binding, the crate's own function, a `dyn` or a fn pointer); a field of
//! a struct JS may make or give, or a nullable one; what a binding, another
//! crate's function, a call through a `dyn` or a type parameter, or a
//! nullable function gives; and a nullable thread-local. Unknown is a
//! source, so the answer is sound by construction.
//! What an exit gives, `?? undefined`, still flows on as what it was given:
//! no answer here leans on a lowering having written it, which a function
//! re-exported as what it calls, or a struct JSX makes, may not.

use crate::collections::{IndexMap, IndexSet};
use crate::lower::Body;
use crate::lower::analysis::fn_pointers::FnPointers;
use crate::lower::bindings;
use crate::lower::recognition::{Recognition, StdItem, is_std_def};
use crate::lower::representation::{NoneForm, NonePlace, none_form};
use rustc_abi::FieldIdx;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_hir::def::DefKind;
use rustc_middle::mir::{self, AggregateKind, Const, Local, Operand, Place, PlaceElem, Rvalue, StatementKind};
use rustc_middle::mir::{RETURN_PLACE, TerminatorKind};
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;

/// The places that may hold a `None` JS gave, as the crate's bodies were
/// found: each body's locals, by its def and local, and each field, by
/// its struct's, closure's or coroutine's def and index.
#[derive(Default)]
pub(in crate::lower) struct JsNones {
    locals: IndexMap<(DefId, Local), u8>,
    fields: IndexMap<(DefId, FieldIdx), u8>,
    /// Each body found, whose locals are answered; any other's may hold one.
    scanned: IndexSet<DefId>,
    /// The props of the crate's components, and each struct their
    /// flattened fields hold, which React gives the component alone.
    props: IndexSet<DefId>,
}

impl JsNones {
    /// Whether `place`, read in `body`, may hold a `None` JS gave: its local,
    /// or a field of a struct, a closure or a coroutine it reads through
    /// (`read_nodes`), may; and any place of a body not found.
    pub(in crate::lower) fn may_hold_null<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        body: &mir::Body<'tcx>,
        place: Place<'tcx>,
    ) -> bool {
        self.sources_of(tcx, body, place) != 0
    }

    /// Every possible `null` was explicitly written as `js::null()`.
    /// Other writes may contribute `undefined` or a value.
    pub(in crate::lower) fn holds_written_null<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        body: &mir::Body<'tcx>,
        place: Place<'tcx>,
    ) -> bool {
        self.sources_of(tcx, body, place) == Source::WrittenNull as u8
    }

    /// Whether `place`, read in `body`, a `Some`'s payload, may itself be
    /// JS's `null`, a `None` JS gave: what JS threw, a `JsError`, of a
    /// caught `null` (ADR 0030, #1939). A payload of another type is no
    /// `None`, whatever `None`s it holds.
    pub(in crate::lower) fn payload_may_be_null<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        body: &mir::Body<'tcx>,
        place: Place<'tcx>,
    ) -> bool {
        let ty = crate::lower::representation::payload_through(tcx, place.ty(body, tcx).ty);
        matches!(ty.kind(), ty::Adt(adt, _) if crate::lower::recognition::is_js_error_type(tcx, adt.did()))
            && self.may_hold_null(tcx, body, place)
    }

    /// The kinds of source (`Source`) whose `None` `place`, read in `body`,
    /// may hold, each a bit; every kind of a body not found.
    pub(in crate::lower) fn sources_of<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        body: &mir::Body<'tcx>,
        place: Place<'tcx>,
    ) -> u8 {
        let def = body.source.def_id();
        if !self.scanned.contains(&def) || place.local.as_usize() >= body.local_decls.len() {
            return Source::Unseen as u8;
        }
        let fields =
            fields_read(tcx, body, place).map(|(of, field, _)| self.fields.get(&(of, field)).copied().unwrap_or(0));
        self.locals.get(&(def, place.local)).copied().unwrap_or(0) | fields.fold(0, |all, kinds| all | kinds)
    }
}

/// What a source may have of JS's `None`, `null`, each a bit of what a
/// place may hold (`JsNones::sources_of`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub(in crate::lower) enum Source {
    /// A nullable place's, whose `None` is `null` as declared: a parameter,
    /// a field, a function's result, a thread-local (ADR 0861).
    Nullable = 1,
    /// What a binding gives that declares neither form (ADR 0298).
    Binding = 2,
    /// What JS gives a function it calls, a parameter.
    Given = 4,
    /// What isn't seen: a closure's or a function's parameter of callers
    /// not seen.
    Unseen = 8,
    /// A call's result whose callee isn't seen, a `dyn`'s or a pointer's.
    Called = 16,
    /// What std gives of what isn't known, a type parameter's or a `dyn`'s.
    Unknown = 32,
    /// Another crate's: its result, a field of its struct, a static.
    Foreign = 64,
    /// A program's explicit `js::null()` (ADR 0935).
    WrittenNull = 128,
}

/// Whether an `Option` of `ty` has one `None` JS may give apart, `null`
/// beside `undefined`, which an exit gives as Rust's, `?? undefined`: not
/// one whose `Some` is boxed, `$some`, as its payload may look like `None`,
/// which JS gives no `null` for; nor one of a value of any shape,
/// `Option<&Unknown>` or `Option<&dyn IntoUnknown>`, whose `undefined` and
/// `null` are each a value of JS's own, passed on as given (rd275, #443).
pub(in crate::lower) fn one_none<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> bool {
    let ty::Adt(adt, args) = ty.kind() else { return false };
    if !tcx.is_lang_item(adt.did(), LangItem::Option) {
        return false;
    }
    let payload = args.type_at(0);
    !crate::lower::representation::may_look_like_none(tcx, payload) && any_value(tcx, payload).is_none()
}

/// What a value of `payload`, an `Option`'s, of any shape is: `Unknown`'s,
/// a type of any shape (`declarations::any_shape`), whose `None` is JS's
/// `undefined` itself; or a `dyn` of what any value is, `dyn IntoUnknown`,
/// `impl<T> IntoUnknown for T`, or of what a value of any shape is. JS's
/// `null` may be the `Some` of either. Not `JsError`'s, of any shape too,
/// whose `None` a caught `null` is (ADR 0030): the one answer of whose
/// `None` is trusted `undefined`, which this owns (#1905).
pub(in crate::lower) fn any_value<'tcx>(tcx: TyCtxt<'tcx>, payload: Ty<'tcx>) -> Option<AnyValue> {
    let any_shape = |ty: Ty<'tcx>| {
        matches!(ty.peel_refs().kind(), ty::Adt(of, _) if crate::lower::declarations::any_shape(tcx, of.did())
            && !crate::lower::recognition::is_js_error_type(tcx, of.did()))
    };
    if any_shape(payload) {
        return Some(AnyValue::Shape);
    }
    let of_any = match payload.peel_refs().kind() {
        ty::Dynamic(traits, ..) => traits.principal_def_id().is_some_and(|t| {
            tcx.all_impls(t).any(|imp| {
                let of = tcx.type_of(imp).instantiate_identity().skip_normalization();
                crate::lower::recognition::known_by_bounds(of) || any_shape(of)
            })
        }),
        _ => false,
    };
    of_any.then_some(AnyValue::Dyn)
}

/// The kind of a value of any shape (`any_value`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(in crate::lower) enum AnyValue {
    /// `Unknown`'s, a type of any shape's.
    Shape,
    /// A `dyn` of what any value is, `dyn IntoUnknown`.
    Dyn,
}

/// Whether what `def` returns leaves for JS as an `Option` with one `None`
/// (`one_none`), as `undefined`: a function JS calls (`js_facing`), not a
/// nullable one's, whose `None` is `null` (ADR 0841), nor an `async`
/// one's, whose promise its body's coroutine settles. Each write of its
/// result is given `?? undefined` where it may hold `null`.
pub(in crate::lower) fn returns_one_none(
    tcx: TyCtxt<'_>,
    foreign: &crate::lower::library::Foreign<'_, '_>,
    def: DefId,
) -> bool {
    def.is_local()
        && crate::lower::analysis::js_facing(tcx, foreign, def)
        && none_form(tcx, NonePlace::Result(def)) == NoneForm::Undefined
        && !tcx.asyncness(def).is_async()
        && one_none(
            tcx,
            tcx.instantiate_bound_regions_with_erased(tcx.fn_sig(def).instantiate_identity().skip_normalization())
                .output(),
        )
}

impl JsNones {
    /// Whether a write of `field`, a struct's JS may see, of `ty`, gives it
    /// an `Option` with one `None` (`one_none`) where it holds `None` as
    /// `undefined`, a plain `Option` field, declared one, not a type
    /// parameter's: it's given `?? undefined` where it may hold `null`, so
    /// what JS sees of the object holds Rust's. A struct JS may see is `pub`,
    /// which a declaration names, or another crate's, a binding's
    /// dictionary; not std's, nor a private one, a guard's, which only Rust
    /// reads, nor the props of the crate's component, which React gives the
    /// component alone, each read of its own an exit asks of.
    pub(in crate::lower) fn field_takes_one_none<'tcx>(
        &self,
        tcx: TyCtxt<'tcx>,
        field: &ty::FieldDef,
        ty: Ty<'tcx>,
    ) -> bool {
        let of = tcx.parent(field.did);
        !self.props.contains(&of) && field_takes_one_none(tcx, field, ty)
    }
}

fn field_takes_one_none<'tcx>(tcx: TyCtxt<'tcx>, field: &ty::FieldDef, ty: Ty<'tcx>) -> bool {
    let declared = tcx.type_of(field.did).instantiate_identity().skip_normalization();
    let of = tcx.parent(field.did);
    let std = crate::lower::recognition::is_std_crate(tcx, of) || crate::lower::recognition::from_sysroot(tcx, of);
    !std && (!of.is_local() || crate::lower::declarations::js_names(tcx, of))
        && matches!(declared.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option))
        && one_none(tcx, ty)
        && none_form(tcx, NonePlace::Field(field, Some(ty))) == NoneForm::Undefined
}

/// Each field `place` reads through, read in `body`: of a struct, by its
/// def, a closure's or a coroutine's capture, by the closure's, each with
/// the field's type; and of a struct, its `FieldDef`.
fn fields_read<'b, 'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &'b mir::Body<'tcx>,
    place: Place<'tcx>,
) -> impl Iterator<Item = (DefId, FieldIdx, Ty<'tcx>)> + 'b {
    place.iter_projections().filter_map(move |(base, elem)| {
        let PlaceElem::Field(field, ty) = elem else { return None };
        let base = base.ty(body, tcx);
        let of = match base.ty.kind() {
            ty::Adt(adt, _) if adt.is_struct() && base.variant_index.is_none() => adt.did(),
            ty::Closure(of, _) | ty::Coroutine(of, _) | ty::CoroutineClosure(of, _) => *of,
            _ => return None,
        };
        Some((of, field, ty))
    })
}

/// The `Option`s a value of a type may hold, by what each's payload points
/// at (`Graph::pointee`), and whether it may hold what isn't known
/// (`Graph::options`).
#[derive(Default)]
struct Options<'tcx> {
    types: IndexSet<Ty<'tcx>>,
    unknown: bool,
}

/// The props of the crate's components, each one's first parameter's
/// struct (`bindings::is_component`), and each struct a flattened field of
/// one holds, which React gives the component alone.
fn component_props<'tcx>(tcx: TyCtxt<'tcx>, bodies: &IndexMap<DefId, &mir::Body<'tcx>>) -> IndexSet<DefId> {
    let mut props = IndexSet::default();
    let mut work: Vec<Ty<'tcx>> = (bodies.iter())
        .filter(|&(&def, _)| def.is_local() && bindings::is_component(tcx, def))
        .filter(|(_, body)| body.arg_count >= 1)
        .map(|(_, body)| body.local_decls[Local::from_usize(1)].ty)
        .collect();
    while let Some(ty) = work.pop() {
        let ty::Adt(adt, args) = *ty.peel_refs().kind() else {
            continue;
        };
        if !adt.is_struct() || !adt.did().is_local() || !props.insert(adt.did()) {
            continue;
        }
        for (i, field) in adt.non_enum_variant().fields.iter().enumerate() {
            if bindings::is_flatten_field(tcx, ty.peel_refs(), i) {
                work.push(field.ty(tcx, args).skip_normalization());
            }
        }
    }
    props
}

/// A place a value of the crate's may be held in.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Node {
    Local(DefId, Local),
    Field(DefId, FieldIdx),
    /// A static's, or a thread-local's key's, which holds its storage.
    Item(DefId),
}

/// The crate's bodies, `bodies` and each initializer of `everywhere`, as
/// they're found: each place that may hold a `None` JS gave.
pub(super) fn found<'tcx>(
    recognition: &Recognition<'_, 'tcx>,
    everywhere: &[&Body<'tcx>],
    pointers: &FnPointers<'tcx>,
) -> JsNones {
    let bodies: IndexMap<DefId, &mir::Body<'tcx>> = (everywhere.iter())
        .filter_map(|body| Some((body.def_id, &body.mir.as_ref()?.body)))
        .collect();
    let promoted: IndexMap<DefId, &rustc_index::IndexVec<mir::Promoted, mir::Body<'tcx>>> = (everywhere.iter())
        .filter_map(|body| Some((body.def_id, &body.mir.as_ref()?.promoted)))
        .collect();
    let mut graph = Graph {
        recognition,
        pointers,
        bodies: &bodies,
        promoted: &promoted,
        nodes: IndexSet::default(),
        next: Vec::new(),
        sources: Vec::new(),
        holds: IndexMap::default(),
        shares: IndexMap::default(),
        escaped: IndexSet::default(),
    };
    for (&def, &body) in &bodies {
        graph.body(def, body);
    }
    graph.seed();
    let tainted = graph.solve();
    let mut nones = JsNones {
        scanned: bodies.keys().copied().collect(),
        props: component_props(recognition.tcx, &bodies),
        ..JsNones::default()
    };
    for (i, node) in graph.nodes.iter().enumerate() {
        if tainted[i] == 0 {
            continue;
        }
        match *node {
            Node::Local(def, local) => _ = nones.locals.insert((def, local), tainted[i]),
            Node::Field(of, field) => _ = nones.fields.insert((of, field), tainted[i]),
            Node::Item(_) => {}
        }
    }
    nones
}

struct Graph<'r, 'a, 'tcx> {
    recognition: &'r Recognition<'a, 'tcx>,
    pointers: &'r FnPointers<'tcx>,
    bodies: &'r IndexMap<DefId, &'r mir::Body<'tcx>>,
    /// Each body's promoted constants, a `&KEY`'s.
    promoted: &'r IndexMap<DefId, &'r rustc_index::IndexVec<mir::Promoted, mir::Body<'tcx>>>,
    nodes: IndexSet<Node>,
    /// Each node's successors, which it flows into.
    next: Vec<Vec<u32>>,
    /// The nodes something not seen may write, and of what (`Source`).
    sources: Vec<(u32, Source)>,
    /// The `Option`s a value of each type may hold (`options`).
    holds: IndexMap<Ty<'tcx>, std::rc::Rc<Options<'tcx>>>,
    /// Whether a value of each type shares what it points at (`shares`).
    shares: IndexMap<Ty<'tcx>, bool>,
    /// The closures and functions given where their callers aren't seen.
    escaped: IndexSet<DefId>,
}

impl<'tcx> Graph<'_, '_, 'tcx> {
    fn tcx(&self) -> TyCtxt<'tcx> {
        self.recognition.tcx
    }

    /// `node`'s index, of a value of `ty`, where it may hold an `Option`;
    /// a nullable field, or another crate's struct's, and a nullable static
    /// or thread-local's key, or another crate's, a source.
    fn id(&mut self, node: Node, ty: Ty<'tcx>) -> Option<u32> {
        if let Some(i) = self.nodes.get_index_of(&node) {
            return Some(i as u32);
        }
        if !self.holds(ty) {
            return None;
        }
        let (i, _) = self.nodes.insert_full(node);
        self.next.push(Vec::new());
        let tcx = self.tcx();
        let source = match node {
            Node::Local(..) => None,
            Node::Field(of, field) => match tcx.def_kind(of) {
                // A std struct's is what std's code wrote, read through what
                // holds it, which says.
                DefKind::Struct if self.recognition.in_sysroot(of) => None,
                DefKind::Struct => {
                    let def = &tcx.adt_def(of).non_enum_variant().fields[field];
                    // A field of an object JS or another crate made is read
                    // through what holds it, which what gave it, a source,
                    // flows to.
                    match none_form(tcx, NonePlace::Field(def, None)) != NoneForm::Undefined {
                        true => Some(Source::Nullable),
                        false => None,
                    }
                }
                _ => None,
            },
            Node::Item(item) => {
                if none_form(tcx, NonePlace::Local(item)).stores_null() {
                    Some(Source::Nullable)
                } else if !item.is_local() {
                    Some(Source::Foreign)
                } else {
                    None
                }
            }
        };
        if let Some(source) = source {
            self.sources.push((i as u32, source));
        }
        Some(i as u32)
    }

    /// Whether a value of `ty` may hold an `Option` (`options`).
    fn holds(&mut self, ty: Ty<'tcx>) -> bool {
        let options = self.options(ty);
        options.unknown || !options.types.is_empty()
    }

    /// Whether a value of `from` may hold an `Option` a value of `to` holds,
    /// by its type (`options`): what a call of std's gives back of what it's
    /// given, which a fresh `None` of std's, `position()`'s, isn't.
    fn carries(&mut self, from: Ty<'tcx>, to: Ty<'tcx>) -> bool {
        let (from, to) = (self.options(from), self.options(to));
        from.unknown || to.unknown || from.types.iter().any(|ty| to.types.contains(ty))
    }

    /// The `Option` types a value of `ty` may hold, through its fields,
    /// what it points at and what it's generic over, and whether it may
    /// hold what isn't known, a type parameter's, a `dyn`'s or an opaque
    /// type's.
    fn options(&mut self, ty: Ty<'tcx>) -> std::rc::Rc<Options<'tcx>> {
        let ty = self.tcx().erase_and_anonymize_regions(ty);
        if let Some(options) = self.holds.get(&ty) {
            return options.clone();
        }
        let mut options = Options::default();
        self.options_in(ty, &mut IndexSet::default(), &mut options);
        let options = std::rc::Rc::new(options);
        self.holds.insert(ty, options.clone());
        options
    }

    /// What a value of `ty` is as an `Option`'s payload a call of std's may
    /// give back as another's, `as_ref()`'s, `as_deref()`'s, `copied()`'s:
    /// what it points at, through every reference, `Box`, `Rc`, `Arc` and
    /// `Pin`, and a `String`'s `str`, a `Vec`'s slice.
    fn pointee(&self, mut ty: Ty<'tcx>) -> Ty<'tcx> {
        let tcx = self.tcx();
        loop {
            ty = crate::lower::representation::payload_through(tcx, ty);
            ty = match *ty.kind() {
                ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::String) => tcx.types.str_,
                ty::Adt(adt, args) if is_std_def(tcx, adt.did(), StdItem::Vec) => Ty::new_slice(tcx, args.type_at(0)),
                _ => return tcx.erase_and_anonymize_regions(ty),
            };
        }
    }

    /// `options` of `ty` into `found`, each type `seen` met once: a
    /// recursive type's, and a chain of types each holding the one before
    /// four times.
    fn options_in(&mut self, ty: Ty<'tcx>, seen: &mut IndexSet<Ty<'tcx>>, found: &mut Options<'tcx>) {
        let tcx = self.tcx();
        if let Some(options) = self.holds.get(&ty) {
            found.unknown |= options.unknown;
            found.types.extend(options.types.iter().copied());
            return;
        }
        if !seen.insert(ty) {
            return;
        }
        match *ty.kind() {
            ty::Adt(adt, args) => {
                if tcx.is_lang_item(adt.did(), LangItem::Option) {
                    found.types.insert(self.pointee(args.type_at(0)));
                }
                // What JS threw holds the `None` of an `Option` of it: a
                // caught `null` is one (ADR 0030), so an `Option` made of
                // it, `Some(e)`, `.err()`, `Option::from(e)`, may hold
                // JS's `null` (#1939).
                if crate::lower::recognition::is_js_error_type(tcx, adt.did()) {
                    found.types.insert(ty);
                }
                for t in args.types() {
                    self.options_in(t, seen, found);
                }
                for field in adt.all_fields() {
                    let field = tcx.erase_and_anonymize_regions(field.ty(tcx, args).skip_normalization());
                    self.options_in(field, seen, found);
                }
            }
            _ if let Some((t, _)) = crate::lower::pointers::raw_pointee(tcx, ty) => self.options_in(t, seen, found),
            ty::Ref(_, t, _) | ty::Slice(t) | ty::Array(t, _) | ty::Pat(t, _) => self.options_in(t, seen, found),
            ty::Tuple(items) => {
                for t in items {
                    self.options_in(t, seen, found);
                }
            }
            ty::Closure(_, args) => self.options_in(args.as_closure().tupled_upvars_ty(), seen, found),
            // A binding's function, JS's, as what isn't known: what holds it
            // holds what JS calls with.
            ty::FnDef(f, _) if bindings::is_binding(tcx, f) => found.unknown = true,
            ty::Bool
            | ty::Char
            | ty::Int(_)
            | ty::Uint(_)
            | ty::Float(_)
            | ty::Str
            | ty::Never
            | ty::FnDef(..)
            | ty::FnPtr(..) => {}
            _ => found.unknown = true,
        }
    }

    /// Whether a value of `ty` shares what it points at, which a write
    /// through a copy of it changes: a reference, a raw pointer, an `Rc`,
    /// an `Arc` or a `Weak`, or what isn't known.
    fn shares(&mut self, ty: Ty<'tcx>) -> bool {
        let ty = self.tcx().erase_and_anonymize_regions(ty);
        if let Some(&shares) = self.shares.get(&ty) {
            return shares;
        }
        let shares = self.shares_in(ty, &mut IndexSet::default());
        self.shares.insert(ty, shares);
        shares
    }

    /// `shares`, of the types `seen` this asking has met, as `holds_in`
    /// asks: a `&mut` or a `*mut`, or a shared reference, a `*const`, an
    /// `Rc`, an `Arc` or a `Weak` of what a write through it may change,
    /// what isn't `Freeze`, a cell's; what holds one, through its fields;
    /// and what isn't known.
    fn shares_in(&mut self, ty: Ty<'tcx>, seen: &mut IndexSet<Ty<'tcx>>) -> bool {
        let tcx = self.tcx();
        if let Some(&shares) = self.shares.get(&ty) {
            return shares;
        }
        if !seen.insert(ty) {
            return false;
        }
        let changes = |of: Ty<'tcx>| {
            of.walk()
                .any(|arg| arg.as_type().is_some_and(crate::lower::recognition::known_by_bounds))
                || !of.is_freeze(tcx, self.recognition.typing_env)
        };
        if let Some((of, mutability)) = crate::lower::pointers::raw_pointee(tcx, ty) {
            return mutability == ty::Mutability::Mut || changes(of);
        }
        match *ty.kind() {
            _ if crate::lower::recognition::known_by_bounds(ty) => true,
            ty::Ref(_, _, ty::Mutability::Mut) => true,
            ty::Ref(_, of, ty::Mutability::Not) => changes(of),
            ty::Dynamic(..) | ty::Foreign(_) => true,
            ty::Adt(adt, args)
                if is_std_def(tcx, adt.did(), StdItem::Rc)
                    || is_std_def(tcx, adt.did(), StdItem::Arc)
                    || crate::lower::recognition::weak_pointee(tcx, ty).is_some() =>
            {
                changes(args.type_at(0))
            }
            ty::Adt(adt, args) => {
                args.types().any(|t| self.shares_in(t, seen))
                    || adt.all_fields().any(|field| {
                        let field = tcx.erase_and_anonymize_regions(field.ty(tcx, args).skip_normalization());
                        self.shares_in(field, seen)
                    })
            }
            ty::Slice(t) | ty::Array(t, _) | ty::Pat(t, _) => self.shares_in(t, seen),
            ty::Tuple(items) => items.iter().any(|t| self.shares_in(t, seen)),
            ty::Closure(_, args) => self.shares_in(args.as_closure().tupled_upvars_ty(), seen),
            ty::Bool
            | ty::Char
            | ty::Int(_)
            | ty::Uint(_)
            | ty::Float(_)
            | ty::Str
            | ty::Never
            | ty::FnDef(..)
            | ty::FnPtr(..) => false,
            _ => true,
        }
    }

    fn edge(&mut self, from: u32, to: u32) {
        if from != to {
            self.next[from as usize].push(to);
        }
    }

    /// What each of `from` holds flows into each of `to`, and back where
    /// `both`.
    fn flow(&mut self, from: &[u32], to: &[u32], both: bool) {
        for &f in from {
            for &t in to {
                self.edge(f, t);
                if both {
                    self.edge(t, f);
                }
            }
        }
    }

    fn local(&mut self, def: DefId, body: &mir::Body<'tcx>, local: Local) -> Option<u32> {
        self.id(Node::Local(def, local), body.local_decls[local].ty)
    }

    /// The nodes a read of `place` reads: its local's, and each field's it
    /// reads through (`fields_read`).
    fn read(&mut self, def: DefId, body: &mir::Body<'tcx>, place: Place<'tcx>) -> Vec<u32> {
        let mut nodes: Vec<u32> = self.local(def, body, place.local).into_iter().collect();
        let fields: Vec<_> = fields_read(self.tcx(), body, place).collect();
        for (of, field, ty) in fields {
            nodes.extend(self.id(Node::Field(of, field), ty));
        }
        nodes
    }

    /// The nodes a write of `place` writes: the last field it writes
    /// through, which every read of it reads, or else its local's.
    fn written(&mut self, def: DefId, body: &mir::Body<'tcx>, place: Place<'tcx>) -> Vec<u32> {
        match fields_read(self.tcx(), body, place).last() {
            Some((of, field, ty)) => self.id(Node::Field(of, field), ty).into_iter().collect(),
            None => self.local(def, body, place.local).into_iter().collect(),
        }
    }

    /// The nodes `operand` reads: a place's, or a static's or a
    /// thread-local's key's a constant refers to; a constant's own value,
    /// none, as rustc made it.
    fn operand(&mut self, def: DefId, body: &mir::Body<'tcx>, operand: &Operand<'tcx>) -> Vec<u32> {
        match operand {
            Operand::Copy(place) | Operand::Move(place) => self.read(def, body, *place),
            // A binding's function as a value, `&text` or `text`, JS's: a
            // callback it's made, called with what may hold `null`, is given
            // Rust's (`ones_given_to_a_callback`).
            Operand::Constant(constant)
                if let ty::FnDef(f, _) = *constant.ty().peel_refs().kind()
                    && bindings::is_binding(self.tcx(), f) =>
            {
                let (i, new) = self.nodes.insert_full(Node::Item(f));
                if new {
                    self.next.push(Vec::new());
                    self.sources.push((i as u32, Source::Binding));
                }
                vec![i as u32]
            }
            Operand::Constant(constant) => {
                let ty = constant.ty();
                (self.item_of(def, constant))
                    .and_then(|item| self.id(Node::Item(item), ty))
                    .into_iter()
                    .collect()
            }
            Operand::RuntimeChecks(_) => Vec::new(),
        }
    }

    /// The static or the thread-local's key `constant` refers to, a
    /// promoted `&KEY`'s too.
    fn item_of(&self, def: DefId, constant: &mir::ConstOperand<'tcx>) -> Option<DefId> {
        let tcx = self.tcx();
        if let Some(item) = constant.check_static_ptr(tcx) {
            return Some(item);
        }
        let Const::Unevaluated(uv, _) = constant.const_ else {
            return None;
        };
        let key = match uv.promoted {
            None => uv.def,
            Some(promoted) => {
                let referred = crate::lower::mir::referred_constant(self.promoted.get(&def)?.get(promoted)?)?;
                let Const::Unevaluated(uv, _) = referred.const_ else {
                    return None;
                };
                uv.def
            }
        };
        crate::lower::analysis::cell_flow::thread_local_key(tcx, key).then_some(key)
    }

    /// Each closure, coroutine and function with a body of the crate's
    /// that a value of `ty` is or holds.
    fn callables(&self, ty: Ty<'tcx>) -> Vec<DefId> {
        let mut found = Vec::new();
        for arg in ty.walk() {
            let Some(t) = arg.as_type() else { continue };
            if let ty::Closure(of, _) | ty::Coroutine(of, _) | ty::CoroutineClosure(of, _) | ty::FnDef(of, _) =
                *t.kind()
                && self.bodies.contains_key(&of)
                && !found.contains(&of)
            {
                found.push(of);
            }
        }
        found
    }

    /// Each closure and function a value of `ty` holds, given where its
    /// caller isn't seen.
    fn escape(&mut self, ty: Ty<'tcx>) {
        for of in self.callables(ty) {
            self.escaped.insert(of);
        }
    }

    /// The nodes of `callee`'s parameters, a closure's past its own
    /// captures, `_1`.
    fn params(&mut self, callee: DefId) -> Vec<u32> {
        let Some(&body) = self.bodies.get(&callee) else {
            return Vec::new();
        };
        let first = match self.tcx().is_closure_like(callee) {
            true => 2,
            false => 1,
        };
        (first..=body.arg_count)
            .filter_map(|i| self.local(callee, body, Local::from_usize(i)))
            .collect()
    }

    fn returned(&mut self, callee: DefId) -> Option<u32> {
        let &body = self.bodies.get(&callee)?;
        self.local(callee, body, RETURN_PLACE)
    }

    fn body(&mut self, def: DefId, body: &mir::Body<'tcx>) {
        // What it returns, a closure or a function, its callers may call
        // where they aren't seen.
        self.escape(body.local_decls[RETURN_PLACE].ty);
        for data in body.basic_blocks.iter() {
            for statement in &data.statements {
                let StatementKind::Assign(assign) = &statement.kind else {
                    continue;
                };
                let (place, rvalue) = &**assign;
                self.assigned(def, body, *place, rvalue);
            }
            let TerminatorKind::Call {
                func,
                args,
                destination,
                ..
            } = &data.terminator().kind
            else {
                continue;
            };
            let args: Vec<&Operand<'tcx>> = args.iter().map(|arg| &arg.node).collect();
            self.call(def, body, (func, &args), *destination);
        }
    }

    fn assigned(&mut self, def: DefId, body: &mir::Body<'tcx>, place: Place<'tcx>, rvalue: &Rvalue<'tcx>) {
        let tcx = self.tcx();
        match rvalue {
            Rvalue::Use(operand, ..)
            | Rvalue::Repeat(operand, _)
            | Rvalue::UnaryOp(_, operand)
            | Rvalue::WrapUnsafeBinder(operand, _) => {
                let from = self.operand(def, body, operand);
                let to = self.written(def, body, place);
                let both = self.shares(operand.ty(body, tcx));
                self.flow(&from, &to, both);
            }
            Rvalue::Cast(_, operand, _) => {
                self.escape(operand.ty(body, tcx));
                let from = self.operand(def, body, operand);
                let to = self.written(def, body, place);
                self.flow(&from, &to, true);
            }
            Rvalue::CopyForDeref(of) => {
                let from = self.read(def, body, *of);
                let to = self.written(def, body, place);
                self.flow(&from, &to, true);
            }
            // A reference reads what it points at, and where a write through
            // it may change that, a `&mut`'s or a cell's, writes it.
            Rvalue::Ref(_, _, of) | Rvalue::RawPtr(_, of) | Rvalue::Reborrow(_, _, of) => {
                let from = self.read(def, body, *of);
                let to = self.written(def, body, place);
                self.flow(&from, &to, false);
                if self.shares(place.ty(body, tcx).ty) {
                    let back = self.written(def, body, *of);
                    self.flow(&to, &back, false);
                }
            }
            Rvalue::ThreadLocalRef(item) => {
                let ty = place.ty(body, tcx).ty;
                let from: Vec<u32> = self.id(Node::Item(*item), ty).into_iter().collect();
                let to = self.written(def, body, place);
                self.flow(&from, &to, true);
            }
            Rvalue::Aggregate(kind, operands) => {
                let fields: Option<DefId> = match **kind {
                    AggregateKind::Adt(did, _, _, _, None) if tcx.adt_def(did).is_struct() => Some(did),
                    AggregateKind::Closure(of, _)
                    | AggregateKind::Coroutine(of, _)
                    | AggregateKind::CoroutineClosure(of, _) => Some(of),
                    _ => None,
                };
                for (i, operand) in operands.iter_enumerated() {
                    let ty = operand.ty(body, tcx);
                    let to: Vec<u32> = match fields {
                        Some(of) => self.id(Node::Field(of, i), ty).into_iter().collect(),
                        None => self.written(def, body, place),
                    };
                    let from = self.operand(def, body, operand);
                    let both = self.shares(ty);
                    self.flow(&from, &to, both);
                }
            }
            Rvalue::BinaryOp(..) | Rvalue::Discriminant(_) => {}
        }
    }

    /// A call of `func` with `args`, its result written to `destination`.
    fn call(
        &mut self,
        def: DefId,
        body: &mir::Body<'tcx>,
        (func, args): (&Operand<'tcx>, &[&Operand<'tcx>]),
        destination: Place<'tcx>,
    ) {
        let tcx = self.tcx();
        let dest = self.written(def, body, destination);
        let given: Vec<Vec<u32>> = args.iter().map(|arg| self.operand(def, body, arg)).collect();
        let tys: Vec<Ty<'tcx>> = args.iter().map(|arg| arg.ty(body, tcx)).collect();
        let Some((callee, generic_args)) = func.const_fn_def() else {
            // Through a fn pointer: what it gives is unknown.
            return self.unseen(&dest, &tys);
        };
        if bindings::is_binding(tcx, callee) {
            for &ty in &tys {
                self.escape(ty);
            }
            // What it writes through a `&mut` it's given, JS's.
            for (written, ty) in given.iter().zip(&tys) {
                if matches!(ty.kind(), ty::Ref(_, _, ty::Mutability::Mut)) {
                    self.sources.extend(written.iter().map(|&node| (node, Source::Binding)));
                }
            }
            // A binding that declares its result `T | undefined`, of what
            // holds no `Option`, gives Rust's `None`.
            let output = destination.ty(body, tcx).ty;
            let trusted = none_form(tcx, NonePlace::CallResult(callee)) == NoneForm::Undefined
                && matches!(output.kind(), ty::Adt(_, args) if !self.holds(args.type_at(0)));
            if !trusted {
                let source = match none_form(tcx, NonePlace::CallResult(callee)) {
                    NoneForm::Null if bindings::is_js_null(tcx, callee) => Source::WrittenNull,
                    NoneForm::Null => Source::Nullable,
                    _ => Source::Binding,
                };
                self.sources.extend(dest.iter().map(|&node| (node, source)));
            }
            return;
        }
        // What the call runs: a trait's method's impl, where it resolves.
        let mut target = callee;
        if tcx.trait_of_assoc(callee).is_some() {
            let typing_env = ty::TypingEnv::post_analysis(tcx, def);
            match crate::lower::traits::resolve(tcx, typing_env, callee, generic_args) {
                Ok(Some(instance)) if !matches!(instance.def, ty::InstanceKind::Virtual(..)) => {
                    target = instance.def_id();
                }
                _ => return self.unseen(&dest, &tys),
            }
            // A JS object's `Deref` to the class it extends, `Error`'s to
            // `JsError`: the object itself (ADR 0024), which holds what it's
            // given (#2112).
            if crate::lower::recognition::is_js_object_deref(tcx, tcx.parent(target))
                && let Some(from) = given.first()
            {
                let both = self.shares(destination.ty(body, tcx).ty);
                self.flow(from, &dest, both);
                return;
            }
            // A closure's or a function's, called through `Fn`: given the
            // tuple of its arguments.
            if !self.bodies.contains_key(&target)
                && tcx
                    .trait_of_assoc(target)
                    .and_then(|t| tcx.fn_trait_kind_from_def_id(t))
                    .is_some()
            {
                let called = match *generic_args.type_at(0).peel_refs().kind() {
                    ty::Closure(of, _) | ty::FnDef(of, _) if self.bodies.contains_key(&of) => of,
                    ty::FnDef(of, _) if bindings::is_binding(tcx, of) => {
                        self.sources.extend(dest.iter().map(|&node| (node, Source::Binding)));
                        return;
                    }
                    ty::Closure(..) | ty::FnDef(..) => {
                        return self.std(&dest, (&given, &tys), destination.ty(body, tcx).ty);
                    }
                    _ => return self.unseen(&dest, &tys),
                };
                let params = self.params(called);
                for (from, &ty) in given.iter().zip(&tys).skip(1) {
                    let both = self.shares(ty);
                    self.flow(from, &params, both);
                }
                let returned: Vec<u32> = self.returned(called).into_iter().collect();
                let both = self.shares(destination.ty(body, tcx).ty);
                self.flow(&returned, &dest, both);
                return;
            }
        }
        if let Some(&callee_body) = self
            .bodies
            .get(&target)
            .filter(|_| !self.recognition.in_sysroot(target))
        {
            for &ty in &tys {
                self.escape(ty);
            }
            for (i, (from, &ty)) in given.iter().zip(&tys).enumerate() {
                if i + 1 > callee_body.arg_count {
                    break;
                }
                let to: Vec<u32> = self
                    .local(target, callee_body, Local::from_usize(i + 1))
                    .into_iter()
                    .collect();
                let both = self.shares(ty);
                self.flow(from, &to, both);
            }
            if crate::lower::calls::gives_null_for_none(tcx, target) {
                self.sources.extend(dest.iter().map(|&node| (node, Source::Nullable)));
            }
            let returned: Vec<u32> = self.returned(target).into_iter().collect();
            let both = self.shares(destination.ty(body, tcx).ty);
            self.flow(&returned, &dest, both);
            return;
        }
        if self.recognition.in_sysroot(target) {
            return self.std(&dest, (&given, &tys), destination.ty(body, tcx).ty);
        }
        // Another crate's: what it gives, and writes through a `&mut`, is
        // unknown.
        for (written, ty) in given.iter().zip(&tys) {
            if matches!(ty.kind(), ty::Ref(_, _, ty::Mutability::Mut)) {
                self.sources.extend(written.iter().map(|&node| (node, Source::Foreign)));
            }
        }
        for &ty in &tys {
            self.escape(ty);
        }
        self.sources.extend(dest.iter().map(|&node| (node, Source::Foreign)));
    }

    /// A call whose callee isn't seen: what it gives may be anything, and
    /// each closure or function it's given its callers aren't seen.
    fn unseen(&mut self, dest: &[u32], tys: &[Ty<'tcx>]) {
        for &ty in tys {
            self.escape(ty);
        }
        self.sources.extend(dest.iter().map(|&node| (node, Source::Called)));
    }

    /// A call of std's, given `given` of `tys`, its result of `output`
    /// written to `dest`: what each argument holds, its result holds, and
    /// so does each argument that points at something; a closure or a
    /// function it's given is given what each holds, and what it returns
    /// the call's result and each such holds. Where it's given what isn't
    /// known, a `dyn`'s, an opaque type's or a type parameter's, which may
    /// hold a closure it calls, its result is unknown.
    fn std(&mut self, dest: &[u32], (given, tys): (&[Vec<u32>], &[Ty<'tcx>]), output: Ty<'tcx>) {
        let all: Vec<u32> = given.iter().flatten().copied().collect();
        let both = self.shares(output);
        for (from, &ty) in given.iter().zip(tys) {
            if self.carries(ty, output) {
                self.flow(from, dest, both);
            }
        }
        let mut pointing: Vec<u32> = Vec::new();
        for (nodes, &ty) in given.iter().zip(tys) {
            if self.shares(ty) {
                pointing.extend(nodes.iter().copied());
            }
        }
        for (from, &ty) in given.iter().zip(tys) {
            for (to, &to_ty) in given.iter().zip(tys) {
                if self.shares(to_ty) && self.carries(ty, to_ty) {
                    self.flow(from, to, true);
                }
            }
        }
        let mut called = Vec::new();
        for &ty in tys.iter().chain([&output]) {
            for of in self.callables(ty) {
                if !called.contains(&of) {
                    called.push(of);
                }
            }
        }
        for of in called {
            let params = self.params(of);
            self.flow(&all, &params, true);
            let returned: Vec<u32> = self.returned(of).into_iter().collect();
            self.flow(&returned, dest, true);
            self.flow(&returned, &pointing, true);
        }
        let unknown = tys.iter().chain([&output]).any(|ty| {
            ty.walk().any(|arg| {
                arg.as_type().is_some_and(|t| {
                    crate::lower::recognition::known_by_bounds(t) || matches!(t.kind(), ty::Dynamic(..))
                })
            })
        });
        if unknown {
            self.sources.extend(dest.iter().map(|&node| (node, Source::Unknown)));
        }
    }

    /// The sources besides those found where they're read: each parameter
    /// of what JS or a consumer calls, of a nullable one, and of each
    /// closure and function given where its callers aren't seen; and what a
    /// thread-local's `init` gives, its key's.
    fn seed(&mut self) {
        let tcx = self.tcx();
        let bodies: Vec<(DefId, &mir::Body<'tcx>)> = self.bodies.iter().map(|(&d, &b)| (d, b)).collect();
        for (def, body) in bodies {
            if tcx
                .opt_parent(def)
                .is_some_and(|key| crate::lower::analysis::cell_flow::thread_local_key(tcx, key))
            {
                let key = tcx.opt_parent(def).expect("its key");
                let ty = tcx.type_of(key).instantiate_identity().skip_normalization();
                if let (Some(returned), Some(key)) = (self.returned(def), self.id(Node::Item(key), ty)) {
                    self.flow(&[returned], &[key], true);
                }
            }
            if !def.is_local() {
                continue;
            }
            let closure = tcx.is_closure_like(def);
            let facing = !closure
                && matches!(tcx.def_kind(def), DefKind::Fn | DefKind::AssocFn)
                && (crate::lower::analysis::js_facing(tcx, self.recognition.foreign, def)
                    || crate::lower::library::reachable(tcx, def));
            let nullable = match closure {
                true => Vec::new(),
                false => crate::lower::representation::null_params(tcx, def),
            };
            let escaped = self.escaped.contains(&def);
            let first = if closure { 2 } else { 1 };
            for i in first..=body.arg_count {
                let local = Local::from_usize(i);
                let ty = body.local_decls[local].ty;
                // What the crate alone makes, its own object JS gives back,
                // holds what its fields do.
                let own = matches!(ty.peel_refs().kind(), ty::Adt(adt, _)
                    if adt.is_struct() && adt.did().is_local() && self.pointers.only_the_crate_makes(adt.did()));
                let source = match () {
                    () if nullable.contains(&(i - 1)) => Some(Source::Nullable),
                    () if facing && !own => Some(Source::Given),
                    () if escaped => Some(Source::Unseen),
                    () => None,
                };
                if let Some(source) = source
                    && let Some(node) = self.local(def, body, local)
                {
                    self.sources.push((node, source));
                }
            }
        }
    }

    /// Each node a source reaches, of each kind of source that does.
    fn solve(&self) -> Vec<u8> {
        let mut tainted = vec![0u8; self.nodes.len()];
        let mut work: Vec<(u32, u8)> = Vec::new();
        for &(source, kind) in &self.sources {
            let kind = kind as u8;
            if tainted[source as usize] & kind == 0 {
                tainted[source as usize] |= kind;
                work.push((source, kind));
            }
        }
        while let Some((node, kind)) = work.pop() {
            for &next in &self.next[node as usize] {
                if tainted[next as usize] & kind == 0 {
                    tainted[next as usize] |= kind;
                    work.push((next, kind));
                }
            }
        }
        tainted
    }
}
