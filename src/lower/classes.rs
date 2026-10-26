//! A JS class (ADR 0761): a struct and its impl marked
//! `#[rust_js::class]` are a constructor function and its prototype, as
//! JS wrote a class before `class`, `function Component(props) { this.props
//! = props; }` and `Component.prototype.setState = function (..) { .. }`.
//! The one answer to whether a type is a class and what each of its impl's
//! items is of it; what's refused of one; and its functions' JS, `this`
//! where Rust has `self`.

use crate::js::{self, Expr, ExprKind, Pattern, Prop, Stmt, StmtKind};
use crate::lower::Body;
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
    /// A function of no `self`: the constructor's own, `Component.from(..)`.
    Static,
    /// An associated `const`: the prototype's property,
    /// `Component.prototype.isReactComponent = {}`.
    Const,
}

fn marked(tcx: TyCtxt<'_>, def_id: DefId) -> bool {
    tcx.get_attrs_by_path(def_id, &[Symbol::intern("rust_js"), Symbol::intern("class")])
        .next()
        .is_some()
}

/// Is `adt` a JS class, a struct marked `#[rust_js::class]`?
pub(crate) fn is_class(tcx: TyCtxt<'_>, adt: DefId) -> bool {
    tcx.def_kind(adt) == DefKind::Struct && marked(tcx, adt)
}

/// The class an inherent impl is of, if it's a class's.
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
    let imp = tcx.inherent_impl_of_assoc(def_id)?;
    class_of_impl(tcx, imp)?;
    Some(match tcx.def_kind(def_id) {
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
    class_of_impl(tcx, tcx.inherent_impl_of_assoc(def_id)?)
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
pub(super) fn reject_misuse<'tcx>(tcx: TyCtxt<'tcx>, bodies: &[&Body<'tcx>]) -> bool {
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
                    if matches!(member(tcx, item), Some(Member::Method | Member::Const)) && fields.contains(&named) {
                        refuse(
                            item_span,
                            &format!(
                                "has no method or constant named as its field, `{named}`, which `this.{named}` would hide: name one otherwise"
                            ),
                        );
                    }
                    match member(tcx, item) {
                        Some(Member::Method) => {
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
            // impl js::Defined`.
            DefKind::Impl { of_trait: true } => {
                let self_ty = tcx.type_of(id).instantiate_identity().skip_normalization();
                if let ty::Adt(adt, _) = self_ty.kind()
                    && is_class(tcx, adt.did())
                    && !tcx.associated_item_def_ids(id).is_empty()
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
    valid
}

/// `function` (ADR 0761) of a class's member, lowered as its impl's
/// function: a constructor's `return { props, .. }` its `this`'s fields
/// set, `this.props = props;`. A method's `self` is `this` already, made
/// so before tidy (`receiver_as_this`). `Err` of a constructor that
/// returns what's no struct literal.
pub(super) fn as_member(member: Member, class: &str, function: &mut js::Function) -> Result<(), &'static str> {
    match member {
        Member::Constructor => set_this(&mut function.body),
        Member::Method => {
            this_outside_accessors(class, &mut function.body);
            Ok(())
        }
        Member::Static | Member::Const => Ok(()),
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
    body: &mut [Stmt],
    booleans: &mut std::collections::HashSet<String>,
) -> Result<(), &'static str> {
    if member(tcx, function) != Some(Member::Method) {
        return Ok(());
    }
    let receiver = match params.first() {
        Some(Pattern::Name(name)) => name.clone(),
        _ => return Err("a method whose `self` is no parameter"),
    };
    params.remove(0);
    js::each_expr_mut(body, &mut |e| {
        if matches!(&e.kind, ExprKind::Var(v) if *v == receiver) {
            e.kind = ExprKind::This;
        }
    });
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

/// Each `return { a, b }` of `body` its fields set on `this`, `this.a =
/// a; this.b = b;`, and `return;` where it's not the body's last; not a
/// closure's, whose `return` is its own.
fn set_this(body: &mut Vec<Stmt>) -> Result<(), &'static str> {
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
                kind: ExprKind::Object(props, ..),
                ..
            })) = stmt.kind
            else {
                unreachable!("a struct literal returned")
            };
            for prop in props {
                match prop {
                    Prop::Field(name, value) => {
                        // Where its value is, which its comment is before.
                        let at = if value.span.is_none() { span } else { value.span };
                        let field = Expr::member(Expr::this(), name);
                        out.push(StmtKind::Assign(field, value).at(at));
                    }
                    _ => fine = Err("a constructor whose struct literal has a field of no name"),
                }
            }
            if Some(i + 1) != last {
                out.push(StmtKind::Return(None).at(span));
            }
        }
        *list = out;
    });
    fine
}
