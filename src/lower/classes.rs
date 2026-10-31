//! A JS class (ADR 0761): a struct and its impl marked
//! `#[rust_js::class]` are a constructor function and its prototype, as
//! JS wrote a class before `class`, `function Component(props) { this.props
//! = props; }` and `Component.prototype.setState = function (..) { .. }`.
//! The one answer to whether a type is a class and what each of its impl's
//! items is of it; what's refused of one; and its functions' JS, `this`
//! where Rust has `self`.

use crate::js::{self, Expr, ExprKind, Pattern, Prop, Stmt, StmtKind};
use crate::lower::Body;
use crate::lower::recognition::{StdItem, is_std_def, opt_std_item};
use rustc_hir::def::DefKind;
use rustc_middle::thir;
use rustc_middle::ty::{self, TyCtxt};
use rustc_span::Symbol;
use rustc_span::def_id::DefId;

/// The name of a class's constructor, Rust's own for one, `fn new(..) ->
/// Self`: the crate's item, not std's.
const CONSTRUCTOR: &str = "new";

/// What an item of a class's impl is of the class.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Member {
    /// `fn new(..) -> Self`: the constructor function, `function
    /// Component(props) { this.props = props; }`, called `new Component(..)`.
    Constructor,
    /// A method of `&self` or `&mut self`: the prototype's, `function (..)
    /// { .. }` reading `this`, called `component.setState(..)`.
    Method,
    /// A method of `&self` alone marked `#[rust_js::getter]`: the
    /// prototype's getter, `Object.defineProperty(QueenPosition.prototype,
    /// "leftDiagonal", { get() { .. } })`, read `position.leftDiagonal`
    /// (#357).
    Getter,
    /// A function of no `self`: the constructor's own, `Component.from(..)`.
    Static,
    /// An associated `const`: the prototype's property,
    /// `Component.prototype.isReactComponent = {}`.
    Const,
    /// A method of `&self` or `&mut self` of a `#[rust_js::object]`'s impl:
    /// a function of its module reading `this`, `function containsNode(node)
    /// { .. }`, each literal of the struct's own property, `{ containsNode
    /// }`, called `scope.containsNode(..)` (ADR 0943).
    Own,
    /// A method of `&self` of a `#[rust_js::namespace]`'s impl: its module's
    /// function, `export function createContainer(..) { .. }`, `self` no
    /// parameter; called by its name where the namespace is named,
    /// `ReactFiberReconciler.create_container(..)`'s `createContainer(..)`,
    /// else through the value, `NoopRenderer.createContainer(..)` (rd292,
    /// ADR 1037).
    Namespace,
}

fn marked(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.get_attrs_by_path(def_id, &[Symbol::intern("rust_js"), Symbol::intern("class")])
        .next()
        .is_some()
}

fn marked_object(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.get_attrs_by_path(def_id, &[Symbol::intern("rust_js"), Symbol::intern("object")])
        .next()
        .is_some()
}

fn marked_namespace(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.get_attrs_by_path(def_id, &[Symbol::intern("rust_js"), Symbol::intern("namespace")])
        .next()
        .is_some()
}

/// Is `adt` a module's namespace, a unit struct marked
/// `#[rust_js::namespace]`, whose value is its module's namespace, `import *
/// as ReactFiberReconciler`, and whose impl's methods are the module's
/// functions (rd292, ADR 1037)?
pub(crate) fn is_namespace(tcx: TyCtxt<'_>, adt: DefId) -> bool {
    tcx.def_kind(adt) == DefKind::Struct && marked_namespace(tcx, adt)
}

/// Whether `receiver`, the `self` a call of a namespace's method in `body`
/// gives it, is the namespace as Rust names it, whose call is the function
/// itself, not a value of it JS may have made of another instance of its
/// module, whose call is that value's (ADR 1037): the unit struct's
/// constant, through what MIR holds it in, or the `self` of the namespace's
/// own method `body` is. The one answer of both the lowering and which
/// library functions the crate compiles (ADR 0914, rd311).
pub(crate) fn names_namespace<'tcx>(
    tcx: TyCtxt<'tcx>,
    body: &rustc_middle::mir::Body<'tcx>,
    receiver: &rustc_middle::mir::Operand<'tcx>,
) -> bool {
    use rustc_middle::mir::{Operand, Rvalue, StatementKind};
    let owner = body.source.def_id();
    let mut operand = receiver.clone();
    for _ in 0..body.local_decls.len() {
        let local = match &operand {
            Operand::Constant(_) => return true,
            Operand::Copy(place) | Operand::Move(place) if place.projection.is_empty() => place.local,
            _ => return false,
        };
        if local.as_usize() <= body.arg_count {
            return local.as_usize() == 1
                && !tcx.is_closure_like(owner)
                && member(tcx, owner) == Some(Member::Namespace);
        }
        // Its one write.
        let mut writes = (body.basic_blocks.iter())
            .flat_map(|data| data.statements.iter())
            .filter_map(|statement| match &statement.kind {
                StatementKind::Assign(assign) if assign.0.local == local => Some(&assign.1),
                _ => None,
            });
        let (Some(write), None) = (writes.next(), writes.next()) else {
            return false;
        };
        let called = (body.basic_blocks.iter()).any(|data| {
            matches!(&data.terminator().kind, rustc_middle::mir::TerminatorKind::Call { destination, .. }
                if destination.local == local)
        });
        operand = match write {
            _ if called => return false,
            Rvalue::Use(operand, _) => operand.clone(),
            Rvalue::CopyForDeref(place) => Operand::Copy(*place),
            // A reborrow, `&*r`.
            Rvalue::Ref(_, _, place) if matches!(place.projection[..], [rustc_middle::mir::PlaceElem::Deref]) => {
                Operand::Copy(place.local.into())
            }
            // `&Reconciler` of the struct made where it's borrowed.
            Rvalue::Ref(_, _, place) if place.projection.is_empty() => {
                let made = (body.basic_blocks.iter())
                    .flat_map(|data| data.statements.iter())
                    .filter_map(|statement| match &statement.kind {
                        StatementKind::Assign(assign) if assign.0.local == place.local => Some(&assign.1),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                return match made[..] {
                    [Rvalue::Aggregate(kind, _)] => {
                        matches!(**kind, rustc_middle::mir::AggregateKind::Adt(adt, ..) if is_namespace(tcx, adt))
                    }
                    [Rvalue::Use(Operand::Constant(_), _)] => true,
                    _ => false,
                };
            }
            _ => return false,
        };
    }
    false
}

/// Refuses, at `span`, a call of `id`, a namespace's method, through a
/// value, the module's own function, which takes its type parameters as JS
/// gives them (ADR 0051) and nothing to copy or drop, where the call needs
/// one apart: given what may look like `None` (`nullish`), whose `Some` it
/// leaves out, or a value to copy or drop (ADRs 0914, 1037, rd311).
pub(crate) fn refuse_through(tcx: TyCtxt<'_>, id: DefId, span: rustc_span::Span, nullish: bool) {
    let message = match nullish {
        true => format!(
            "rust-js: `{}` is called through a namespace's value, the module's own function, which takes its type parameters as JS gives them, but this call gives one what may look like `None`, an `Option`, which that function reads apart from Rust's (ADRs 0051, 1037): call it on the namespace as Rust names it, give it what's no `Option`, or `.flatten()` its result where the function only makes its `Option` and returns it",
            tcx.def_path_str(id)
        ),
        false => format!(
            "rust-js: `{}` is called through a namespace's value, its library's own function, which takes nothing to copy or drop what this call gives it (ADRs 0914, 1037): call it on the namespace as Rust names it",
            tcx.def_path_str(id)
        ),
    };
    tcx.dcx().span_err(span, message);
}

/// The namespace an impl is of, if it's one's.
fn namespace_of_impl(tcx: TyCtxt<'_>, imp: DefId) -> Option<DefId> {
    match tcx.type_of(imp).instantiate_identity().skip_normalization().kind() {
        ty::Adt(adt, _) if is_namespace(tcx, adt.did()) => Some(adt.did()),
        _ => None,
    }
}

/// Is `adt` an object of methods, a struct marked `#[rust_js::object]`,
/// whose literal is JS's object literal of its fields and its impl's
/// methods, which read `this` (ADR 0943)?
pub(crate) fn is_object(tcx: TyCtxt<'_>, adt: DefId) -> bool {
    tcx.def_kind(adt) == DefKind::Struct && marked_object(tcx, adt)
}

/// The object of methods an impl is of, if it's one's.
fn object_of_impl(tcx: TyCtxt<'_>, imp: DefId) -> Option<DefId> {
    match tcx.type_of(imp).instantiate_identity().skip_normalization().kind() {
        ty::Adt(adt, _) if is_object(tcx, adt.did()) => Some(adt.did()),
        _ => None,
    }
}

/// The object of methods `def_id`, its impl's method, is of; `None` of any
/// other item.
pub(crate) fn object_of(tcx: TyCtxt<'_>, def_id: DefId) -> Option<DefId> {
    (member(tcx, def_id) == Some(Member::Own))
        .then(|| object_of_impl(tcx, tcx.inherent_impl_of_assoc(def_id)?))
        .flatten()
}

/// The property an object's method `def_id` is of its literals, its name:
/// camelCase, or its `#[rust_js::name]`, `DO_NOT_USE_queryAllNodes`, as a
/// field's is, whatever its function is named in its module (ADR 0943).
pub(crate) fn property(tcx: TyCtxt<'_>, def_id: DefId) -> String {
    crate::lower::bindings::fn_name(tcx, def_id)
}

/// The methods of `adt`, an object of methods, each its literal's own
/// property, in the order its impls write them (ADR 0943); none of what's
/// no object.
pub(crate) fn own_methods(tcx: TyCtxt<'_>, adt: DefId) -> Vec<DefId> {
    if !is_object(tcx, adt) {
        return Vec::new();
    }
    (tcx.inherent_impls(adt).iter())
        .flat_map(|&imp| tcx.associated_item_def_ids(imp).iter().copied())
        .filter(|&item| member(tcx, item) == Some(Member::Own))
        .collect()
}

fn marked_getter(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.get_attrs_by_path(def_id, &[Symbol::intern("rust_js"), Symbol::intern("getter")])
        .next()
        .is_some()
}

/// Is `method`, a class's, one a getter is: of `&self` alone, no type
/// parameters, giving a value.
fn reads_as_getter(tcx: TyCtxt<'_>, method: DefId) -> bool {
    let sig = tcx
        .fn_sig(method)
        .instantiate_identity()
        .skip_normalization()
        .skip_binder();
    matches!(sig.inputs(), [receiver] if matches!(receiver.kind(), ty::Ref(_, _, ty::Mutability::Not)))
        && !sig.output().is_unit()
        && !tcx.asyncness(method).is_async()
        && tcx
            .generics_of(method)
            .own_params
            .iter()
            .all(|p| !p.kind.is_ty_or_const())
}

/// Is `adt` a JS class, a struct marked `#[rust_js::class]`?
pub(crate) fn is_class(tcx: TyCtxt<'_>, adt: DefId) -> bool {
    tcx.def_kind(adt) == DefKind::Struct && marked(tcx, adt)
}

/// The class `imp` is its `Display`'s, `impl fmt::Display for
/// QueenPosition`, whose `fmt` is its prototype's `toString` (#358).
pub(crate) fn displayed_class(tcx: TyCtxt<'_>, imp: DefId) -> Option<DefId> {
    let DefKind::Impl { of_trait: true } = tcx.def_kind(imp) else {
        return None;
    };
    if !is_std_def(tcx, tcx.impl_trait_id(imp), StdItem::Display) {
        return None;
    }
    class_of_impl(tcx, imp)
}

/// The class `def_id` is the `fmt` of the `Display` of, its prototype's
/// `toString` (#358); `None` of any other item.
pub(crate) fn shown_class(tcx: TyCtxt<'_>, def_id: DefId) -> Option<DefId> {
    (tcx.def_kind(def_id) == DefKind::AssocFn)
        .then(|| displayed_class(tcx, tcx.trait_impl_of_assoc(def_id)?))
        .flatten()
}

/// Whether `class` has a `Display`, its prototype's `toString` (#358).
pub(crate) fn has_to_string(tcx: TyCtxt<'_>, class: DefId) -> bool {
    opt_std_item(tcx, StdItem::Display).is_some_and(|display| {
        (tcx.local_trait_impls(display).iter()).any(|imp| displayed_class(tcx, imp.to_def_id()) == Some(class))
    })
}

/// Whether `trait_id` is a trait of classes, the crate's, with items, each
/// impl of which is a class's (rd283): each of its methods, an impl's or
/// the trait's default, is each class's prototype's, `ReactDOMRoot.prototype
/// .render = function (children) { .. }`, never a dictionary's.
pub(crate) fn is_class_trait(tcx: TyCtxt<'_>, trait_id: DefId) -> bool {
    let mut impls = tcx.all_impls(trait_id).peekable();
    trait_id.is_local()
        && tcx.def_kind(trait_id) == DefKind::Trait
        && !tcx.associated_item_def_ids(trait_id).is_empty()
        && impls.peek().is_some()
        && impls.all(|imp| class_of_impl(tcx, imp).is_some())
}

/// The class `imp` is an impl of a trait of classes of (rd283).
pub(crate) fn trait_impl_class(tcx: TyCtxt<'_>, imp: DefId) -> Option<DefId> {
    let DefKind::Impl { of_trait: true } = tcx.def_kind(imp) else {
        return None;
    };
    let class = class_of_impl(tcx, imp)?;
    is_class_trait(tcx, tcx.impl_trait_id(imp)).then_some(class)
}

/// The class whose prototype's method `def_id` is, a method of its impl
/// of a trait of classes (rd283); `None` of any other item.
pub(crate) fn trait_member_class(tcx: TyCtxt<'_>, def_id: DefId) -> Option<DefId> {
    (tcx.def_kind(def_id) == DefKind::AssocFn)
        .then(|| trait_impl_class(tcx, tcx.trait_impl_of_assoc(def_id)?))
        .flatten()
}

/// Of `method`, a trait of classes' method, what each class's call of it
/// runs, its impl's own or the trait's default of its `Self`: what a call of
/// it whose `Self` isn't known, a default's, may be, as it has no impls but
/// these (rd304). `None` of any other.
pub(crate) fn implementations<'tcx>(
    tcx: TyCtxt<'tcx>,
    method: DefId,
) -> Option<Vec<(DefId, ty::GenericArgsRef<'tcx>)>> {
    let trait_id = tcx.trait_of_assoc(method)?;
    if !is_class_trait(tcx, trait_id) {
        return None;
    }
    let found = tcx
        .all_impls(trait_id)
        .map(|imp| match tcx.impl_item_implementor_ids(imp).get(&method) {
            Some(&own) => (own, ty::GenericArgs::identity_for_item(tcx, own)),
            None => {
                let of = tcx.type_of(imp).instantiate_identity().skip_normalization();
                let args = ty::GenericArgs::for_item(tcx, method, |param, _| match param.index {
                    0 => of.into(),
                    _ => tcx.mk_param_from_def(param),
                });
                (method, args)
            }
        });
    Some(found.collect())
}

/// Of `def_id`, an impl's method of a trait of classes that only gives a
/// reference to a field of its `&self`, `fn internal_root(&self) ->
/// &RefCell<..> { &self.internal_root }`: that field, by the path of fields
/// to it and each one's own, read in place wherever it's called, no method
/// of its prototype (rd304, ADR 1035 amended). `None` of any other.
pub(crate) fn field_accessor(tcx: TyCtxt<'_>, def_id: DefId) -> Option<Vec<(rustc_abi::FieldIdx, DefId)>> {
    use rustc_middle::ty::adjustment::{Adjust, DerefAdjustKind};
    trait_member_class(tcx, def_id)?;
    let local = def_id.as_local()?;
    let body = tcx.hir_body_owned_by(local);
    let [this, ..] = body.params else { return None };
    let typeck = tcx.typeck(local);
    let given = body.value.peel_blocks();
    let rustc_hir::ExprKind::AddrOf(rustc_hir::BorrowKind::Ref, rustc_hir::Mutability::Not, mut place) = given.kind
    else {
        return None;
    };
    // Of none, or the reborrow of what it gives back, `&*&self.f`.
    use rustc_middle::ty::adjustment::{AutoBorrow, AutoBorrowMutability};
    match typeck.expr_adjustments(given) {
        [] => {}
        [deref, borrow]
            if matches!(deref.kind, Adjust::Deref(DerefAdjustKind::Builtin))
                && matches!(borrow.kind, Adjust::Borrow(AutoBorrow::Ref(AutoBorrowMutability::Not))) => {}
        _ => return None,
    }
    // Of `self.a.b`, `b` then `a`: each base of a field the struct itself
    // but `self`, of which only a reference's own deref.
    let mut fields = Vec::new();
    loop {
        match place.kind {
            rustc_hir::ExprKind::Field(base, _) => {
                let derefs = typeck.expr_adjustments(base);
                let is_self = matches!(base.kind, rustc_hir::ExprKind::Path(rustc_hir::QPath::Resolved(None, path))
                    if path.res == rustc_hir::def::Res::Local(this.pat.hir_id));
                let adjusted = match derefs {
                    [] => false,
                    [deref] if is_self && matches!(deref.kind, Adjust::Deref(DerefAdjustKind::Builtin)) => true,
                    _ => return None,
                };
                let ty::Adt(adt, _) = typeck.expr_ty_adjusted(base).kind() else {
                    return None;
                };
                if !adt.is_struct() {
                    return None;
                }
                let index = typeck.field_index(place.hir_id);
                fields.push((index, adt.non_enum_variant().fields[index].did));
                if is_self {
                    return adjusted.then(|| fields.into_iter().rev().collect());
                }
                place = base;
            }
            _ => return None,
        }
    }
}

/// The fields a call of `callee`, of `args` in `typing_env`, may give in
/// place, each the last of a `field_accessor`'s path: of the one method it
/// is, or of a `Self` not known, a default's, each of its classes' that
/// gives one (rd304). Empty of any other call.
pub(crate) fn fields_given<'tcx>(
    tcx: TyCtxt<'tcx>,
    typing_env: ty::TypingEnv<'tcx>,
    (callee, args): (DefId, ty::GenericArgsRef<'tcx>),
) -> Vec<DefId> {
    if !(tcx.trait_of_assoc(callee)).is_some_and(|of| is_class_trait(tcx, of)) {
        return Vec::new();
    }
    let may_be = match ty::Instance::try_resolve(tcx, typing_env, callee, args) {
        Ok(Some(instance)) => vec![instance.def_id()],
        Ok(None) => (implementations(tcx, callee).into_iter().flatten())
            .map(|(own, _)| own)
            .collect(),
        Err(_) => Vec::new(),
    };
    (may_be.into_iter())
        .filter_map(|own| field_accessor(tcx, own)?.last().map(|&(_, field)| field))
        .collect()
}

/// The class an impl is of, if it's a class's.
fn class_of_impl(tcx: TyCtxt<'_>, imp: DefId) -> Option<DefId> {
    match tcx.type_of(imp).instantiate_identity().skip_normalization().kind() {
        ty::Adt(adt, _) if is_class(tcx, adt.did()) => Some(adt.did()),
        _ => None,
    }
}

/// What `def_id`, an item of a class's inherent impl, is of the class;
/// `None` of any other item.
pub(crate) fn member(tcx: TyCtxt<'_>, def_id: DefId) -> Option<Member> {
    if !matches!(tcx.def_kind(def_id), DefKind::AssocFn | DefKind::AssocConst { .. }) {
        return None;
    }
    // Its `Display`'s `fmt`, the prototype's `toString` (#358).
    if shown_class(tcx, def_id).is_some() || trait_member_class(tcx, def_id).is_some() {
        return Some(Member::Method);
    }
    let imp = tcx.inherent_impl_of_assoc(def_id)?;
    // An object's method, its own property; its impl's other items are any
    // impl's (ADR 0943).
    if object_of_impl(tcx, imp).is_some() {
        let method = tcx.def_kind(def_id) == DefKind::AssocFn && tcx.associated_item(def_id).is_method();
        return method.then_some(Member::Own);
    }
    // A namespace's method, its module's function (rd292, ADR 1037).
    if namespace_of_impl(tcx, imp).is_some() {
        let method = tcx.def_kind(def_id) == DefKind::AssocFn && tcx.associated_item(def_id).is_method();
        return method.then_some(Member::Namespace);
    }
    class_of_impl(tcx, imp)?;
    Some(match tcx.def_kind(def_id) {
        DefKind::AssocFn if tcx.associated_item(def_id).is_method() && marked_getter(tcx, def_id) => Member::Getter,
        DefKind::AssocFn if tcx.associated_item(def_id).is_method() => Member::Method,
        DefKind::AssocFn if is_constructor_name(tcx.item_name(def_id)) => Member::Constructor,
        DefKind::AssocFn => Member::Static,
        _ => Member::Const,
    })
}

/// The class `def_id`, an item of a class's inherent impl, is of; `None`
/// of any other item.
pub(crate) fn class_of(tcx: TyCtxt<'_>, def_id: DefId) -> Option<DefId> {
    member(tcx, def_id)?;
    (shown_class(tcx, def_id).or_else(|| trait_member_class(tcx, def_id)))
        .or_else(|| class_of_impl(tcx, tcx.inherent_impl_of_assoc(def_id)?))
}

/// The constructor of `adt`, a class: its `fn new(..) -> Self`; `None` of
/// what's no class.
pub(crate) fn constructor_of(tcx: TyCtxt<'_>, adt: DefId) -> Option<DefId> {
    if !is_class(tcx, adt) {
        return None;
    }
    (tcx.inherent_impls(adt).iter())
        .flat_map(|&imp| tcx.associated_item_def_ids(imp).iter().copied())
        .find(|&item| member(tcx, item) == Some(Member::Constructor))
}

/// The module a class `adt`, the crate's, is in: its constructor
/// function's, its `fn new`'s impl's, where its declaration is too, though
/// its struct is another module's, React's `Fiber` of ReactInternalTypes
/// made in ReactFiber (ADR 0761, amended); `None` of what's no class.
pub(crate) fn home(tcx: TyCtxt<'_>, adt: DefId) -> Option<rustc_span::def_id::LocalModId> {
    Some(tcx.parent_module_from_def_id(constructor_of(tcx, adt)?.as_local()?))
}

/// The class `module`'s item `item` declares: a class's struct, where its
/// constructor function is too, or the impl of its constructor, where its
/// struct is another module's (ADR 0761, amended).
pub(crate) fn declared_by(tcx: TyCtxt<'_>, item: DefId, module: rustc_span::def_id::LocalModId) -> Option<DefId> {
    let adt = match tcx.def_kind(item) {
        DefKind::Struct => item,
        DefKind::Impl { of_trait: false } => class_of_impl(tcx, item).filter(|&adt| {
            constructor_of(tcx, adt).and_then(|new| tcx.inherent_impl_of_assoc(new)) == Some(item)
                && adt
                    .as_local()
                    .is_some_and(|struct_| tcx.parent_module_from_def_id(struct_) != module)
        })?,
        _ => return None,
    };
    (is_class(tcx, adt) && home(tcx, adt).is_none_or(|home| home == module)).then_some(adt)
}

/// The JS name of `adt`, a class: its constructor function's, its
/// struct's `#[rust_js::name]`, `FiberNode` of React's `Fiber`, as a
/// binding's class is named (ADR 0761, amended); `None` of what's no class.
pub(crate) fn name(tcx: TyCtxt<'_>, adt: DefId) -> Option<String> {
    is_class(tcx, adt).then(|| crate::lower::bindings::class_name(tcx, adt))
}

fn is_constructor_name(name: Symbol) -> bool {
    name.as_str() == CONSTRUCTOR
}

/// A class's member's JS name, before it's made unique among its class's:
/// a `const`'s camelCase, a prototype's property, `isReactComponent` of
/// `IS_REACT_COMPONENT`, as a method's is; `None` of what's no class's
/// `const`, named as any item is.
pub(crate) fn const_name(tcx: TyCtxt<'_>, def_id: DefId) -> Option<String> {
    (member(tcx, def_id) == Some(Member::Const)).then(|| {
        let named = crate::lower::bindings::fn_name(tcx, def_id);
        match named == tcx.item_name(def_id).as_str() {
            true => crate::lower::camel_case(&named.to_lowercase()),
            false => named,
        }
    })
}

/// Report what a crate does with a class that rust-js refuses (ADR 0761).
/// False if there was one.
pub(super) fn reject_misuse<'tcx>(
    recognition: &crate::lower::recognition::Recognition<'_, 'tcx>,
    bodies: &[&Body<'tcx>],
) -> bool {
    let tcx = recognition.tcx;
    let by_id: std::collections::HashMap<DefId, &Body<'tcx>> = bodies.iter().map(|&body| (body.def_id, body)).collect();
    let mut valid = true;
    let mut refuse = |span, what: &str| {
        tcx.dcx()
            .span_err(span, format!("rust-js: a `#[rust_js::class]` {what} (ADR 0761)"));
        valid = false;
    };
    for def_id in tcx.hir_crate_items(()).definitions() {
        let id = def_id.to_def_id();
        let span = tcx.def_span(def_id);
        match tcx.def_kind(def_id) {
            DefKind::Struct if marked(tcx, id) => {
                let variant = tcx.adt_def(id).non_enum_variant();
                if variant.ctor.is_some() {
                    refuse(
                        span,
                        "is a struct of named fields, `struct Component { props: .. }`, its `this`'s",
                    );
                }
                if tcx.generics_of(id).own_params.iter().any(|p| p.kind.is_ty_or_const()) {
                    refuse(
                        span,
                        "has no type or const parameters: JS's prototype is one for every value",
                    );
                }
                if constructor_of(tcx, id).is_none() {
                    refuse(span, "is made by its constructor, `fn new(..) -> Self`, in its impl");
                }
            }
            DefKind::Enum | DefKind::Union | DefKind::Fn | DefKind::Trait if marked(tcx, id) => {
                refuse(span, "is a struct and its impl, `struct Component { .. }`");
            }
            _ if marked_getter(tcx, id) && class_of(tcx, id).is_none() => {
                refuse(
                    span,
                    "getter is a `#[rust_js::class]`'s method, `fn left_diagonal(&self) -> i32`",
                );
            }
            DefKind::Impl { of_trait: false } => {
                let of_class = class_of_impl(tcx, id).is_some();
                match (of_class, marked(tcx, id)) {
                    (true, false) => refuse(span, "struct's impl is marked `#[rust_js::class]` too"),
                    (false, true) => refuse(span, "impl is of a struct marked `#[rust_js::class]`"),
                    _ => {}
                }
                let Some(class) = class_of_impl(tcx, id) else {
                    continue;
                };
                let fields: Vec<String> = (tcx.adt_def(class).non_enum_variant().fields.iter())
                    .map(|field| crate::lower::bindings::field_key(tcx, field))
                    .collect();
                for &item in tcx.associated_item_def_ids(id) {
                    let item_span = tcx.def_span(item);
                    // An instance's own property hides its prototype's: a
                    // method named as a field is never called.
                    let named = const_name(tcx, item).unwrap_or_else(|| crate::lower::bindings::fn_name(tcx, item));
                    if matches!(member(tcx, item), Some(Member::Method | Member::Getter | Member::Const))
                        && fields.contains(&named)
                    {
                        refuse(
                            item_span,
                            &format!(
                                "has no method or constant named as its field, `{named}`, which `this.{named}` would hide: name one otherwise"
                            ),
                        );
                    }
                    if marked_getter(tcx, item)
                        && !(member(tcx, item) == Some(Member::Getter) && reads_as_getter(tcx, item))
                    {
                        refuse(
                            item_span,
                            "getter is a method of `&self` alone that gives a value, `fn left_diagonal(&self) -> i32`, read `position.leftDiagonal`",
                        );
                    }
                    match member(tcx, item) {
                        Some(Member::Method | Member::Getter) => {
                            let receiver = tcx
                                .fn_sig(item)
                                .instantiate_identity()
                                .skip_normalization()
                                .skip_binder()
                                .inputs()[0];
                            if !matches!(receiver.kind(), ty::Ref(_, inner, _) if matches!(inner.kind(), ty::Adt(..))) {
                                refuse(
                                    item_span,
                                    "method takes `&self` or `&mut self`, `this`, not `self` by value",
                                );
                            }
                        }
                        Some(Member::Constructor) => {
                            let sig = tcx
                                .fn_sig(item)
                                .instantiate_identity()
                                .skip_normalization()
                                .skip_binder();
                            let own = tcx.type_of(id).instantiate_identity().skip_normalization();
                            if sig.output() != own {
                                refuse(item_span, "constructor, `fn new(..) -> Self`, gives `Self`");
                            }
                        }
                        _ => {}
                    }
                }
            }
            // A trait of classes' impl, in its class's module, where its
            // methods are the prototype's (rd283).
            DefKind::Impl { of_trait: true } if let Some(class) = trait_impl_class(tcx, id) => {
                if home(tcx, class) != Some(tcx.parent_module_from_def_id(def_id)) {
                    refuse(
                        span,
                        "implements the crate's trait in the module its constructor is in, where its prototype is",
                    );
                }
                // An instance's own property hides its prototype's: a
                // method named as a field is never called. Not one that
                // gives a field, which is that field, no method (rd304).
                let fields: Vec<String> = (tcx.adt_def(class).non_enum_variant().fields.iter())
                    .map(|field| crate::lower::bindings::field_key(tcx, field))
                    .collect();
                for &item in tcx.associated_item_def_ids(tcx.impl_trait_id(id)) {
                    let named = crate::lower::bindings::fn_name(tcx, item);
                    let own = tcx.impl_item_implementor_ids(id).get(&item).copied();
                    if fields.contains(&named) && !own.is_some_and(|own| field_accessor(tcx, own).is_some()) {
                        refuse(
                            span,
                            &format!(
                                "has no method or constant named as its field, `{named}`, which `this.{named}` would hide: name one otherwise"
                            ),
                        );
                    }
                }
                // A default that takes rest arguments is called so on each
                // class: its own method would be given them spread.
                for &own in tcx.associated_item_def_ids(id) {
                    if tcx
                        .trait_item_of(own)
                        .is_some_and(|item| crate::lower::bindings::takes_rest(tcx, item))
                    {
                        refuse(
                            tcx.def_span(own),
                            "takes a trait's method that takes rest arguments as the trait's default gives it, not its own",
                        );
                    }
                }
            }
            DefKind::Trait if is_class_trait(tcx, id) => {
                let name = tcx.item_name(id);
                let generics = tcx.generics_of(id);
                let own = generics.own_params.iter().skip(usize::from(generics.has_self));
                if own.clone().any(|p| p.kind.is_ty_or_const()) {
                    refuse(
                        span,
                        &format!(
                            "implements `{name}`, a trait of no type or const parameters: a prototype's method is one for every value"
                        ),
                    );
                }
                let sizedness = |t: DefId| {
                    [
                        rustc_hir::attrs::lang_items::LangItem::Sized,
                        rustc_hir::attrs::lang_items::LangItem::MetaSized,
                        rustc_hir::attrs::lang_items::LangItem::PointeeSized,
                    ]
                    .into_iter()
                    .any(|item| tcx.is_lang_item(t, item))
                };
                let supertraits = tcx.explicit_super_clauses_of(id).skip_binder();
                if (supertraits.iter())
                    .any(|(clause, _)| clause.as_trait_clause().is_none_or(|t| !sizedness(t.def_id())))
                {
                    refuse(
                        span,
                        &format!(
                            "implements `{name}`, a trait of no supertraits: its methods are its prototype's alone"
                        ),
                    );
                }
                for &item in tcx.associated_item_def_ids(id) {
                    let item_span = tcx.def_span(item);
                    let method = tcx.def_kind(item) == DefKind::AssocFn && tcx.associated_item(item).is_method();
                    let receiver = method.then(|| {
                        tcx.fn_sig(item)
                            .instantiate_identity()
                            .skip_normalization()
                            .skip_binder()
                            .inputs()[0]
                    });
                    if !receiver.is_some_and(|receiver| matches!(receiver.kind(), ty::Ref(..))) {
                        refuse(
                            item_span,
                            &format!(
                                "implements `{name}`, whose items are methods of `&self` or `&mut self`, its prototype's"
                            ),
                        );
                    } else if tcx.generics_of(item).own_params.iter().any(|p| p.kind.is_ty_or_const()) {
                        refuse(
                            item_span,
                            &format!(
                                "implements `{name}`, whose methods have no type or const parameters, as a prototype's method is one"
                            ),
                        );
                    }
                }
            }
            // A trait's methods would be a dictionary's, given the value,
            // not the prototype's: only a marker's, of no items, `unsafe
            // impl js::Defined`, and `Display`'s, its `toString` (#358); and
            // a `PartialEq` and a `Hash` of its address, `ptr::eq` and
            // `ptr::hash`, of one equal only to itself, which JS's `===` and
            // a `Set` are of an object, with no JS of their own (ADR 0864;
            // ADR 0761, amended).
            DefKind::Impl { of_trait: true } if displayed_class(tcx, id).is_none() => {
                let self_ty = tcx.type_of(id).instantiate_identity().skip_normalization();
                let trait_id = tcx.impl_trait_id(id);
                let identity = (tcx.lang_items().eq_trait() == Some(trait_id)
                    || opt_std_item(tcx, StdItem::Hash) == Some(trait_id))
                    && crate::lower::std_impls::equal_only_to_itself(recognition, &by_id, self_ty);
                if let ty::Adt(adt, _) = self_ty.kind()
                    && is_class(tcx, adt.did())
                    && !tcx.associated_item_def_ids(id).is_empty()
                    && !identity
                {
                    let name = tcx.def_path_str(tcx.impl_trait_id(id));
                    refuse(
                        span,
                        &format!(
                            "implements no trait but a marker's, of no items, not `{name}`: its methods are its prototype's"
                        ),
                    );
                }
            }
            _ => {}
        }
    }
    // A trait of classes bounds no type parameter and is no `dyn`'s: its
    // methods are each class's prototype's, called on the value, with no
    // dictionary for generic code (rd283).
    for def_id in tcx.hir_crate_items(()).definitions() {
        let id = def_id.to_def_id();
        if matches!(tcx.def_kind(id), DefKind::Trait | DefKind::Impl { of_trait: true })
            || tcx
                .opt_parent(id)
                .is_some_and(|parent| tcx.def_kind(parent) == DefKind::Trait)
            || !matches!(
                tcx.def_kind(id),
                DefKind::Fn | DefKind::AssocFn | DefKind::Struct | DefKind::Enum | DefKind::Impl { .. }
            )
        {
            continue;
        }
        for (clause, span) in tcx.clauses_of(id).clauses {
            if let Some(bound) = clause.as_trait_clause()
                && is_class_trait(tcx, bound.def_id())
                && !matches!(bound.self_ty().skip_binder().kind(), ty::Adt(adt, _) if is_class(tcx, adt.did()))
            {
                let name = tcx.item_name(bound.def_id());
                refuse(
                    *span,
                    &format!(
                        "implements `{name}`, a trait of classes, which bounds no type parameter: call its methods on a class's value, `root.render(..)`"
                    ),
                );
            }
        }
    }
    for body in bodies {
        for expr in body.thir.exprs.iter() {
            let dyn_of = expr.ty.walk().find_map(|part| match part.as_type()?.kind() {
                ty::Dynamic(predicates, ..) => predicates.principal_def_id().filter(|&t| is_class_trait(tcx, t)),
                _ => None,
            });
            if let Some(trait_id) = dyn_of {
                let name = tcx.item_name(trait_id);
                refuse(
                    expr.span,
                    &format!(
                        "implements `{name}`, a trait of classes, of no `dyn`: call its methods on a class's value, `root.render(..)`"
                    ),
                );
                break;
            }
        }
    }
    // A method of a trait of classes that gives a field is that field,
    // read where it's called: no function of its own to hold (rd304).
    for body in bodies {
        let thir = &body.thir;
        let typing_env = ty::TypingEnv::post_analysis(tcx, tcx.typeck_root_def_id(body.def_id));
        let called: std::collections::HashSet<thir::ExprId> = (thir.exprs.iter())
            .filter_map(|expr| match expr.kind {
                thir::ExprKind::Call { fun, .. } => Some(fun),
                _ => None,
            })
            .collect();
        for (id, expr) in thir.exprs.iter_enumerated() {
            if let Some((method, args)) = crate::lower::fn_def(expr.ty)
                && !called.contains(&id)
                && matches!(expr.kind, thir::ExprKind::ZstLiteral { .. })
                && !fields_given(tcx, typing_env, (method, args)).is_empty()
            {
                let name = tcx.item_name(method);
                refuse(
                    expr.span,
                    &format!(
                        "method that gives its field, `{name}`, is that field, read where it's called, `x.{name}()`: no function to hold"
                    ),
                );
            }
        }
    }
    // Made by its constructor alone: a struct literal anywhere else is a
    // plain object, no instance of its prototype.
    for body in bodies {
        let in_constructor = member(tcx, tcx.typeck_root_def_id(body.def_id)) == Some(Member::Constructor);
        for expr in body.thir.exprs.iter() {
            if let thir::ExprKind::Adt(adt) = &expr.kind
                && is_class(tcx, adt.adt_def.did())
                && !(in_constructor
                    && tcx
                        .inherent_impl_of_assoc(tcx.typeck_root_def_id(body.def_id))
                        .and_then(|imp| class_of_impl(tcx, imp))
                        == Some(adt.adt_def.did()))
            {
                let name = tcx.item_name(adt.adt_def.did());
                refuse(
                    expr.span,
                    &format!("is made by its constructor, `{name}::new(..)`, not a struct literal"),
                );
            }
        }
    }
    let objects = reject_object_misuse(tcx);
    reject_namespace_misuse(tcx) && objects && valid
}

/// Report each constructor held as a value of any shape, which a `Class`
/// holds, the one way (ADR 1009): a function constructed, `js::new(js::unknown(&fake), ())`,
/// or whose prototype is read, `js::get(js::unknown(&fake), "prototype")`,
/// of `Class::of(&fake)`; and a `Class` constructed as any value,
/// `js::new(js::unknown(class), ())`, of `js::new(class, ..)`, or of a
/// class held, `class.call_static(..)` of its constructor's binding.
/// False if there was one.
pub(super) fn reject_constructors_as_values<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> bool {
    use crate::lower::body_queries::strip;
    use crate::lower::recognition::{is_class_of, is_js_class, is_untyped_class, js_root_fn};
    let mut valid = true;
    for body in bodies {
        let thir = &body.thir;
        // What's under a borrow, a deref and a coercion, as it's given.
        let given = |mut e: thir::ExprId| loop {
            e = strip(thir, e);
            match thir[e].kind {
                thir::ExprKind::Borrow { arg, .. } | thir::ExprKind::Deref { arg } => e = arg,
                thir::ExprKind::PointerCoercion { source, .. } => e = source,
                _ => break e,
            }
        };
        let called = |e: thir::ExprId| match thir[e].kind {
            thir::ExprKind::Call { fun, ref args, .. } => match *thir[strip(thir, fun)].ty.kind() {
                ty::FnDef(did, generic_args) => Some((js_root_fn(tcx, did)?, generic_args, args)),
                _ => None,
            },
            _ => None,
        };
        for expr in thir.exprs.iter() {
            let thir::ExprKind::Call { fun, ref args, .. } = expr.kind else {
                continue;
            };
            let ty::FnDef(did, fn_args) = *thir[strip(thir, fun)].ty.kind() else {
                continue;
            };
            // A closure is an arrow, which `new` throws of: no constructor.
            if is_class_of(tcx, did) && matches!(fn_args.skip_binder().type_at(1).peel_refs().kind(), ty::Closure(..)) {
                let why = "a closure is an arrow function, which `new` throws of: a constructor is a `fn` item, `Class::of(&f)`, or one `function::with_this` makes, `Class::of(function::with_this(..))`";
                tcx.dcx().span_err(expr.span, format!("rust-js: {why} (ADR 1009)"));
                valid = false;
                continue;
            }
            let Some(name) = js_root_fn(tcx, did) else {
                continue;
            };
            // Its first argument a value of any shape of what `js::unknown`
            // or `js::unknown_of` is given.
            let Some((of, generic_args, _)) = args.first().and_then(|&first| called(given(first))) else {
                continue;
            };
            if !matches!(of.as_str(), "unknown" | "unknown_of") {
                continue;
            }
            let held = generic_args.skip_binder().type_at(0).peel_refs();
            let function = is_function(tcx, held);
            let class = matches!(held.kind(), ty::Adt(adt, _) if is_js_class(tcx, adt.did()));
            let prototype = || {
                args.get(1).is_some_and(|&key| {
                    matches!(&thir[given(key)].kind, thir::ExprKind::Literal { lit, .. }
                        if matches!(lit.node, rustc_ast::LitKind::Str(key, _) if key.as_str() == "prototype"))
                })
            };
            let why = match (name.as_str(), function, class) {
                ("new", true, _) => {
                    "a constructor is held as a `js::Class`, `Class::of(&f)`, which `js::new(class, (a, b))` constructs, not as a value of any shape"
                }
                ("get", true, _) if prototype() => {
                    "a constructor is held as a `js::Class`, whose prototype is `Class::of(&f).prototype()`, not as a value of any shape"
                }
                // An untyped one, a class held, by its constructor's
                // binding (ADR 1027, amended).
                ("new", _, true) if is_untyped_class(tcx, held) => {
                    "a class held, an untyped `js::Class`, is made by its constructor's binding, `class.call_static(webapi::abort_controller::new, ())`, not as a value of any shape"
                }
                ("new", _, true) => {
                    "a `js::Class` is constructed as it is, `js::new(class, ..)`, not as a value of any shape"
                }
                _ => continue,
            };
            tcx.dcx().span_err(expr.span, format!("rust-js: {why} (ADR 1009)"));
            valid = false;
        }
    }
    valid
}

/// Whether `ty` is a function: an item, a closure, a pointer, a `dyn Fn`,
/// or a box of one.
fn is_function<'tcx>(tcx: TyCtxt<'tcx>, ty: ty::Ty<'tcx>) -> bool {
    match ty.kind() {
        ty::FnDef(..) | ty::Closure(..) | ty::FnPtr(..) => true,
        ty::Dynamic(predicates, ..) => predicates.principal_def_id().is_some_and(|did| tcx.is_fn_trait(did)),
        ty::Adt(adt, args) if adt.is_box() => is_function(tcx, args.type_at(0)),
        _ => false,
    }
}

/// Report what a crate does with an object of methods that rust-js
/// refuses (ADR 0943). False if there was one.
fn reject_object_misuse(tcx: TyCtxt<'_>) -> bool {
    let mut valid = true;
    let mut refuse = |span, what: &str| {
        tcx.dcx()
            .span_err(span, format!("rust-js: a `#[rust_js::object]` {what} (ADR 0943)"));
        valid = false;
    };
    for def_id in tcx.hir_crate_items(()).definitions() {
        let id = def_id.to_def_id();
        let span = tcx.def_span(def_id);
        match tcx.def_kind(def_id) {
            DefKind::Struct if marked_object(tcx, id) => {
                if tcx.adt_def(id).non_enum_variant().ctor.is_some() {
                    refuse(span, "is a struct of named fields, `struct ScopeInstance { .. }`");
                }
                let generic = (tcx.generics_of(id).own_params.iter()).any(|param| param.kind.is_ty_or_const());
                if generic {
                    refuse(
                        span,
                        "has no type or const parameters: each of its methods is one function for every value",
                    );
                }
                if marked(tcx, id) {
                    refuse(
                        span,
                        "isn't a `#[rust_js::class]` too: its methods are its own, not a prototype's",
                    );
                }
            }
            DefKind::Enum | DefKind::Union | DefKind::Fn | DefKind::Trait if marked_object(tcx, id) => {
                refuse(span, "is a struct and its impl, `struct ScopeInstance { .. }`");
            }
            DefKind::Impl { of_trait: false } => {
                let of_object = object_of_impl(tcx, id);
                match (of_object.is_some(), marked_object(tcx, id)) {
                    (true, false) => refuse(span, "struct's impl is marked `#[rust_js::object]` too"),
                    (false, true) => refuse(span, "impl is of a struct marked `#[rust_js::object]`"),
                    _ => {}
                }
                let Some(object) = of_object else {
                    continue;
                };
                let fields: Vec<String> = (tcx.adt_def(object).non_enum_variant().fields.iter())
                    .map(|field| crate::lower::bindings::field_key(tcx, field))
                    .collect();
                for &item in tcx.associated_item_def_ids(id) {
                    if member(tcx, item) != Some(Member::Own) {
                        continue;
                    }
                    let item_span = tcx.def_span(item);
                    let named = property(tcx, item);
                    if fields.contains(&named) {
                        refuse(
                            item_span,
                            &format!(
                                "has no method named as its field, `{named}`, two values of one key: name one otherwise"
                            ),
                        );
                    }
                    let receiver = tcx
                        .fn_sig(item)
                        .instantiate_identity()
                        .skip_normalization()
                        .skip_binder()
                        .inputs()[0];
                    if !matches!(receiver.kind(), ty::Ref(_, inner, _) if matches!(inner.kind(), ty::Adt(..))) {
                        refuse(
                            item_span,
                            "method takes `&self` or `&mut self`, `this`, not `self` by value",
                        );
                    }
                }
            }
            _ => {}
        }
    }
    valid
}

/// Report what a crate does with a module's namespace that rust-js
/// refuses (rd292, ADR 1037). False if there was one.
fn reject_namespace_misuse(tcx: TyCtxt<'_>) -> bool {
    let mut valid = true;
    let mut refuse = |span, what: &str| {
        tcx.dcx()
            .span_err(span, format!("rust-js: a `#[rust_js::namespace]` {what} (ADR 1037)"));
        valid = false;
    };
    for def_id in tcx.hir_crate_items(()).definitions() {
        let id = def_id.to_def_id();
        let span = tcx.def_span(def_id);
        match tcx.def_kind(def_id) {
            DefKind::Struct if marked_namespace(tcx, id) => {
                let variant = tcx.adt_def(id).non_enum_variant();
                let generic = (tcx.generics_of(id).own_params.iter()).any(|param| param.kind.is_ty_or_const());
                if variant.ctor_kind() != Some(rustc_hir::def::CtorKind::Const) || generic {
                    refuse(
                        span,
                        "is a unit struct, `pub struct ReactFiberReconciler;`: its value is its module, which holds nothing else",
                    );
                }
                if marked(tcx, id) || marked_object(tcx, id) {
                    refuse(
                        span,
                        "isn't a `#[rust_js::class]` or `#[rust_js::object]` too: its value is its module's namespace",
                    );
                }
            }
            DefKind::Enum | DefKind::Union | DefKind::Fn | DefKind::Trait if marked_namespace(tcx, id) => {
                refuse(
                    span,
                    "is a unit struct and its impl, `pub struct ReactFiberReconciler;`",
                );
            }
            DefKind::Impl { of_trait: false } => {
                let of_namespace = namespace_of_impl(tcx, id);
                match (of_namespace.is_some(), marked_namespace(tcx, id)) {
                    (true, false) => refuse(span, "struct's impl is marked `#[rust_js::namespace]` too"),
                    (false, true) => refuse(span, "impl is of a struct marked `#[rust_js::namespace]`"),
                    _ => {}
                }
                let Some(namespace) = of_namespace else {
                    continue;
                };
                // Its functions are its module's: written where its value is.
                if namespace.as_local().map(|adt| tcx.parent_module_from_def_id(adt))
                    != Some(tcx.parent_module_from_def_id(def_id))
                {
                    refuse(span, "impl is in its struct's module, whose functions its methods are");
                }
                for &item in tcx.associated_item_def_ids(id) {
                    let by_ref = member(tcx, item) == Some(Member::Namespace) && {
                        let sig = tcx
                            .fn_sig(item)
                            .instantiate_identity()
                            .skip_normalization()
                            .skip_binder();
                        matches!(sig.inputs()[0].kind(), ty::Ref(_, _, ty::Mutability::Not))
                    };
                    if !by_ref {
                        refuse(
                            tcx.def_span(item),
                            "has methods of `&self` alone, its module's functions, `pub fn create_container(&self, ..)`",
                        );
                    }
                }
            }
            _ => {}
        }
    }
    valid
}

/// `function` (ADR 0761) of a class's member, lowered as its impl's
/// function: a constructor's `return { props, .. }` its `this`'s fields
/// set, `this.props = props;`. A method's `self` is `this` already, made
/// so before tidy (`receiver_as_this`). `Err` of a constructor that
/// returns what's no struct literal.
pub(super) fn as_member(
    tcx: TyCtxt<'_>,
    def_id: DefId,
    class: &str,
    function: &mut js::Function,
) -> Result<(), &'static str> {
    let Some(member) = member(tcx, def_id) else {
        return Ok(());
    };
    match member {
        // Its instance a handle may be kept on the place of, a field's, as
        // a method's `this` (#359).
        Member::Constructor => set_this(&mut function.body).map(|()| this_outside_accessors(class, &mut function.body)),
        Member::Method | Member::Getter => {
            this_outside_accessors(class, &mut function.body);
            Ok(())
        }
        // Named as its struct is, its function of no type's; exported only
        // where another module names it, which its callers add (ADR 0943).
        Member::Own => {
            let object =
                object_of(tcx, def_id).map_or_else(|| "object".to_owned(), |adt| tcx.item_name(adt).to_string());
            this_outside_accessors(&object, &mut function.body);
            function.export = false;
            Ok(())
        }
        Member::Static | Member::Const | Member::Namespace => Ok(()),
    }
}

/// Where `class`'s member `name` is written, as `member` is of it: its
/// constructor function, its prototype's method or getter; the rest, its
/// statics and constants, written where they are.
pub(super) fn place(member: Member, class: &mut js::Class, name: &str) {
    match member {
        Member::Constructor => class.constructor = name.to_owned(),
        Member::Method => class.prototype.push(name.to_owned()),
        Member::Getter => class.getters.push(name.to_owned()),
        Member::Static | Member::Const | Member::Own | Member::Namespace => {}
    }
}

/// Of `body`, a class's method's: each handle kept on a place of `this`,
/// which preparation writes `{ get value() { return this.reads; }, .. }`,
/// whose `this` is the handle, on the instance named before it, `const
/// counted = this;`, `counted.reads` (#359). Named as its class is, as a
/// method's `self` is.
fn this_outside_accessors(class: &str, body: &mut Vec<Stmt>) {
    let mut kept = false;
    js::each_expr(body, &mut |e| {
        if let ExprKind::Handle(place, _) | ExprKind::Pair(place, _) = &e.kind {
            place.each(&mut |e| kept |= matches!(e.kind, ExprKind::This));
        }
    });
    if !kept {
        return;
    }
    let mut taken = std::collections::HashSet::new();
    js::names_in(body, &mut taken);
    let mut instance = class.to_owned();
    instance[..1].make_ascii_lowercase();
    let instance = crate::lower::fresh_in(&mut taken, &instance);
    js::each_expr_mut(body, &mut |e| {
        if let ExprKind::Handle(place, _) | ExprKind::Pair(place, _) = &mut e.kind {
            place.each_mut(&mut |e| {
                if matches!(e.kind, ExprKind::This) {
                    e.kind = ExprKind::Var(instance.clone());
                }
            });
        }
    });
    body.insert(0, StmtKind::Const(instance, Expr::this()).at(js::Span::NONE));
}

/// Of `function`, a class's method, its `self` parameter, `this` in its
/// body, `params` left without it. Before tidy, which would take a `let`
/// of its name, read no more after, for it (ADR 0650): `let complex_number
/// = ..` of a `ComplexNumber`'s method, whose `self` is named
/// `complexNumber` too, would be `this = ..`. And the paths of `booleans`
/// through it, `comparator.lessThan()`'s `this.lessThan()`, which tidy
/// reads `this`'s by.
pub(super) fn receiver_as_this(
    tcx: TyCtxt<'_>,
    function: DefId,
    params: &mut Vec<Pattern>,
    body: &mut Vec<Stmt>,
    booleans: &mut std::collections::HashSet<String>,
) -> Result<(), &'static str> {
    // A namespace's method has no `self`: what it calls through it is called
    // by its name (rd292, ADR 1037), and nothing else reads it.
    if member(tcx, function) == Some(Member::Namespace) {
        let Some(Pattern::Name(receiver)) = params.first() else {
            return Err("a method whose `self` is no parameter");
        };
        let mut read = false;
        js::visit_stmts(body, &mut |name| read |= name == receiver);
        if read {
            return Err(
                "`#[rust_js::namespace]` method that reads `self` only to call its functions, `self.update_container(..)`: its module has no value of its own (ADR 1037)",
            );
        }
        params.remove(0);
        return Ok(());
    }
    if !matches!(
        member(tcx, function),
        Some(Member::Method | Member::Getter | Member::Own)
    ) {
        return Ok(());
    }
    let receiver = match params.first() {
        Some(Pattern::Name(name)) => name.clone(),
        _ => return Err("a method whose `self` is no parameter"),
    };
    params.remove(0);
    // A function nested in it, `js::function::with_this`'s, has a `this`
    // of its own: it reads `self` as the method's, a `const` of it.
    if js::read_as_this(body, &receiver) {
        body.insert(0, StmtKind::Const(receiver.clone(), Expr::this()).at(js::Span::NONE));
    }
    let through = format!("{receiver}.");
    *booleans = std::mem::take(booleans)
        .into_iter()
        .map(|path| match path.strip_prefix(&through) {
            Some(rest) => format!("this.{rest}"),
            None => path,
        })
        .collect();
    Ok(())
}

/// The assignments to `this` of a constructor's struct literal's fields,
/// `this.a = a;`, each where its value is, or at `span`: not of a field
/// that's `undefined`, a `None`, which an object leaves out (ADR 0280), as
/// JS's constructor function sets no property it hasn't a value of.
fn fields_set(props: Vec<Prop>, layout: &js::ObjectLayout, span: js::Span) -> Result<Vec<Stmt>, &'static str> {
    let mut props = props;
    js::undefined_keys::left_out(&mut props, layout);
    let mut out = Vec::with_capacity(props.len());
    for prop in props {
        match prop {
            Prop::Field(name, value) => {
                // Where its value is, which its comment is before.
                let at = if value.span.is_none() { span } else { value.span };
                out.push(StmtKind::Assign(Expr::member(Expr::this(), name), value).at(at));
            }
            _ => return Err("a constructor whose struct literal has a field of no name"),
        }
    }
    Ok(out)
}

/// Of a constructor's `body`, the variable its last statement returns,
/// made of a struct literal where it's declared and never set again,
/// `let fiber = Fiber { .. }; if on { fiber.x.set(..); } fiber`: that
/// value is `this`, its literal's fields set on it where it's declared,
/// each read of it `this`, `Object.preventExtensions(this)`, and each
/// `return fiber` `return;`, the body's last none (ADR 0761, amended).
fn instance_as_this(body: &mut Vec<Stmt>) -> Result<(), &'static str> {
    let Some(Stmt {
        kind: StmtKind::Return(Some(Expr {
            kind: ExprKind::Var(instance),
            ..
        })),
        ..
    }) = body.last()
    else {
        return Ok(());
    };
    let instance = instance.clone();
    let Some(at) = body.iter().position(|stmt| {
        matches!(&stmt.kind,
            StmtKind::Const(name, value) | StmtKind::Let(name, Some(value))
                if *name == instance && matches!(value.kind, ExprKind::Object(..)))
    }) else {
        return Ok(());
    };
    let mut set_again = false;
    js::each_list(body, &mut |list| {
        set_again |= list.iter().any(
            |stmt| matches!(&stmt.kind, StmtKind::Assign(Expr { kind: ExprKind::Var(v), .. }, _) if *v == instance),
        );
    });
    if set_again {
        return Err("a constructor whose instance is set again, not changed in place");
    }
    let made = body.remove(at);
    let (StmtKind::Const(_, value) | StmtKind::Let(_, Some(value))) = made.kind else {
        unreachable!("the instance's declaration")
    };
    let ExprKind::Object(props, layout) = value.kind else {
        unreachable!("a struct literal")
    };
    let fields = fields_set(props, &layout, made.span)?;
    body.splice(at..at, fields);
    body.pop();
    js::statement_lists(body, &mut |list| {
        for stmt in list.iter_mut() {
            if matches!(&stmt.kind, StmtKind::Return(Some(Expr { kind: ExprKind::Var(v), .. })) if *v == instance) {
                stmt.kind = StmtKind::Return(None);
            }
        }
    });
    if js::read_as_this(body, &instance) {
        return Err("a constructor whose instance a function made in it reads, whose `this` is its own");
    }
    Ok(())
}

/// Each `return { a, b }` of `body` its fields set on `this`, `this.a =
/// a; this.b = b;`, and `return;` where it's not the body's last; not a
/// closure's, whose `return` is its own. Or the instance a variable holds,
/// `instance_as_this`.
fn set_this(body: &mut Vec<Stmt>) -> Result<(), &'static str> {
    instance_as_this(body)?;
    let mut fine = Ok(());
    let mut outermost = true;
    js::statement_lists(body, &mut |list| {
        let last = std::mem::take(&mut outermost).then_some(list.len());
        let mut out = Vec::with_capacity(list.len());
        for (i, stmt) in std::mem::take(list).into_iter().enumerate() {
            if !matches!(&stmt.kind, StmtKind::Return(Some(value)) if matches!(value.kind, ExprKind::Object(..))) {
                if matches!(stmt.kind, StmtKind::Return(Some(_))) {
                    fine = Err("a constructor that gives what's no struct literal, `Self { .. }`");
                }
                out.push(stmt);
                continue;
            }
            let span = stmt.span;
            let StmtKind::Return(Some(Expr {
                kind: ExprKind::Object(props, layout),
                ..
            })) = stmt.kind
            else {
                unreachable!("a struct literal returned")
            };
            match fields_set(props, &layout, span) {
                Ok(fields) => out.extend(fields),
                Err(why) => fine = Err(why),
            }
            if Some(i + 1) != last {
                out.push(StmtKind::Return(None).at(span));
            }
        }
        *list = out;
    });
    fine
}

/// Each default a class takes of a trait of classes, of its impl's
/// dictionary lowered, `body`, which returns its object: a function of
/// each default's arrow, its prototype's method, `this` its `self`; the
/// impl's own methods are its prototype's already (rd283, ADR 1035).
pub(super) fn prototype_defaults(mut body: Vec<Stmt>) -> Vec<js::Function> {
    let Some(Stmt {
        kind: StmtKind::Return(Some(Expr {
            kind: ExprKind::Object(props, _),
            ..
        })),
        ..
    }) = body.pop()
    else {
        unreachable!("a dictionary given nothing returns its object")
    };
    let mut defaults = Vec::new();
    for prop in props {
        let Prop::Field(name, value) = prop else { continue };
        let (is_async, span) = (matches!(value.kind, ExprKind::AsyncArrow(..)), value.span);
        let (ExprKind::Arrow(mut params, mut body) | ExprKind::AsyncArrow(mut params, mut body)) = value.kind else {
            continue;
        };
        if let Some(Pattern::Name(receiver)) = (!params.is_empty()).then(|| params.remove(0))
            && js::read_as_this(&mut body, &receiver)
        {
            body.insert(0, StmtKind::Const(receiver, Expr::this()).at(js::Span::NONE));
        }
        defaults.push(js::Function {
            name,
            params,
            body,
            export: false,
            is_async,
            noinline: false,
            span,
            name_span: js::Span::NONE,
            param_spans: Vec::new(),
        });
    }
    defaults
}

/// `function`, a default the class `owner` of a module of `namespaces`
/// takes, its prototype's method beside its own; or, where a class of the
/// module took one alike before, that class's function, assigned to each
/// (rd283, ADR 1035). `Err` of one named as a method the class has.
pub(super) fn take_default(
    namespaces: &mut [js::Namespace],
    owner: &str,
    function: js::Function,
) -> Result<(), String> {
    let Some(at) = namespaces.iter().position(|n| n.name == owner) else {
        return Ok(());
    };
    if namespaces[at].methods.iter().any(|f| f.name == function.name) {
        return Err(format!(
            "a class's method named as a method its trait gives it, `{}`, which one prototype holds one of",
            function.name
        ));
    }
    let alike = namespaces.iter_mut().find(|n| {
        n.name != owner
            && n.class
                .as_ref()
                .is_some_and(|class| class.prototype.contains(&function.name))
            && n.methods
                .iter()
                .any(|f| f.name == function.name && f.params == function.params && f.body == function.body)
    });
    if let Some(first) = alike
        && let Some(class) = &mut first.class
    {
        match class.shared.iter_mut().find(|(method, _)| *method == function.name) {
            Some((_, others)) => others.push(owner.to_owned()),
            None => class.shared.push((function.name.clone(), vec![owner.to_owned()])),
        }
        return Ok(());
    }
    let namespace = &mut namespaces[at];
    if let Some(class) = &mut namespace.class {
        class.prototype.push(function.name.clone());
    }
    namespace.methods.push(function);
    Ok(())
}
