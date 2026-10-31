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
    if shown_class(tcx, def_id).is_some() {
        return Some(Member::Method);
    }
    let imp = tcx.inherent_impl_of_assoc(def_id)?;
    // An object's method, its own property; its impl's other items are any
    // impl's (ADR 0943).
    if object_of_impl(tcx, imp).is_some() {
        let method = tcx.def_kind(def_id) == DefKind::AssocFn && tcx.associated_item(def_id).is_method();
        return method.then_some(Member::Own);
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
    shown_class(tcx, def_id).or_else(|| class_of_impl(tcx, tcx.inherent_impl_of_assoc(def_id)?))
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
    reject_object_misuse(tcx) && valid
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
        Member::Static | Member::Const => Ok(()),
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
        Member::Static | Member::Const | Member::Own => {}
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
