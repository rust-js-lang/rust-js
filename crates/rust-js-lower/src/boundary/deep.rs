//! The deep checks (#2403 step 2b(b), ADR 1194): a parameter of a value
//! that holds others, a struct, a tuple, an array, a `Vec` or a slice, is
//! checked whole, each value in it by its own type's check, as deep as it
//! goes: `$checkWhole(checkOrder, order, "count(order)")`. A struct's
//! checker is its module's function, made once, which checks it's an
//! object and each field by its key, as `Shape::Object` keys it:
//!
//! ```js
//! function checkOrder(value) {
//!   $checkObject(value);
//!   $checkField(value, "id", $checkU32);
//!   $checkField(value, "items", (value, where, none) => $checkArray(value, where, none, checkItem));
//!   $checkField(value, "note", $checkStr, "nullish");
//! }
//! ```
//!
//! A failure inside the value names where it is, `count(order.items[1].qty)`,
//! built only as it's thrown (`$checkWhole`). Not yet: a generic struct, one
//! keyed by a symbol, an enum with fields, a map, a set.

use rust_js_program::collections::IndexSet;
use rust_js_program::js::{self, Expr};
use rustc_hir::def::DefKind;
use rustc_middle::ty::consts::ConstExt as _;
use rustc_middle::ty::{self, Ty};
use rustc_span::def_id::DefId;

use crate::temporary_names::{Role, declared};

use crate::declarations::contract::Contract;
use crate::{FnCx, Shape};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    /// The checker of a value of `ty` that holds others, which `$checkWhole`
    /// calls with it; none of one it can't check yet.
    pub(super) fn deep_checker(&self, ty: Ty<'tcx>) -> Option<Expr> {
        match self.contract(ty) {
            Contract::Shared(inner) => self.deep_checker(inner),
            // A `Box`, an uncounted `Rc`: the value it points at.
            Contract::Pointee(inner) => self.checker(inner),
            Contract::Vec(item) => Some(array_checker(self.checker(item)?, None)),
            Contract::Items(item) => {
                let length = match ty.kind() {
                    ty::Array(_, length) => length.try_to_target_usize(self.tcx),
                    _ => None,
                };
                Some(array_checker(self.checker(item)?, length))
            }
            Contract::Tuple(items) => {
                let checkers = (items.iter())
                    .map(|item| self.checker(item))
                    .collect::<Option<Vec<_>>>()?;
                Some(arrow_of("$checkTuple", vec![Expr::array(checkers)]))
            }
            // A library's struct too, its checker the consumer's own, made of its
            // fields as rustc's metadata says them, its brand's key its crate's,
            // one registry's (#2403 (Y)).
            Contract::Local(did, args) | Contract::Library(did, args) if self.tcx.def_kind(did) == DefKind::Struct => {
                let reference = self.struct_checker(did)?;
                self.checker_of_use(reference, args)
            }
            Contract::Local(did, args) if self.tcx.def_kind(did) == DefKind::Enum => {
                let reference = self.enum_checker(did)?;
                self.checker_of_use(reference, args)
            }
            // A `&mut`: of a value JS can't change in place, its box, `{
            // value }`, its `value` by the value's check; of an object, the
            // object itself (ADR 0099). Of a generic box, a guard or what's
            // given as it is: not yet.
            Contract::MutRef(inner) => match self.mut_ref_form(inner).checked_through_its_cell()? {
                true => Some(arrow_of("$checkCell", vec![self.checker(inner)?])),
                false => self.checker(inner),
            },
            // A map, a `Map`, of a key that's itself as JS compares it, a
            // number, a string, a `bool` or a `char`: each key, each value. A
            // key a `$KeySet` keeps apart, a tuple's, a struct's, is JS's
            // another way: not yet.
            Contract::Map(key, value) if self.plain_key(key) => {
                Some(arrow_of("$checkMap", vec![self.checker(key)?, self.checker(value)?]))
            }
            Contract::Set(item) if self.plain_key(item) => Some(arrow_of("$checkSet", vec![self.checker(item)?])),
            // A generic type's own type parameter, inside its checker: the
            // checker it's given.
            Contract::Param(param) => {
                let dependencies = self.dependencies.borrow();
                let (_, name) =
                    (dependencies.checker_params.last()?.iter()).find(|(index, _)| *index == param.index)?;
                Some(Expr::var(name))
            }
            // A `Result`, `{ TAG: "Ok" | "Err", _0 }`, inline, of its two types.
            Contract::Tagged(did, args)
                if crate::recognition::is_std_def(self.tcx, did, crate::recognition::StdItem::Result) =>
            {
                let (ok, err) = (self.checker(args.type_at(0))?, self.checker(args.type_at(1))?);
                let value = declared(Role::Value);
                let body = tagged_body(
                    &value,
                    js::TAG,
                    (
                        vec![js::result::name(true).into(), js::result::name(false).into()],
                        Vec::new(),
                    ),
                    vec![
                        (js::result::name(true).into(), vec![(js::positional(0), ok, None)]),
                        (js::result::name(false).into(), vec![(js::positional(0), err, None)]),
                    ],
                );
                Some(Expr::arrow(vec![value.into()], body))
            }
            _ => None,
        }
    }

    /// The checker of a value of `ty`, any kind checked: a helper of the
    /// runtime's, `$checkU32`, or a value's that holds others; called with
    /// the value, where it is and the `None` it may be.
    pub(super) fn checker(&self, ty: Ty<'tcx>) -> Option<Expr> {
        // An `Option` in what's checked, its `None` either JS's.
        if let Contract::Option(inner) = self.contract(ty) {
            // Its `Some` a box where its value might look like `None` (ADR
            // 0051), what the box holds checked.
            let helper = match self.boxed_payload(inner) {
                true => "$checkBoxedOption",
                false => "$checkOption",
            };
            let inner = self.checker(inner)?;
            let (value, place) = (declared(Role::Value), declared(Role::Where));
            let call = Expr::call(Expr::helper(helper), vec![Expr::var(&value), Expr::var(&place), inner]);
            return Some(Expr::arrow(
                vec![value.into(), place.into()],
                vec![js::StmtKind::Return(Some(call)).at(js::Span::NONE)],
            ));
        }
        let check = self.shallow_check(ty)?;
        if let Some(whole) = check.whole {
            return Some(whole);
        }
        Some(match check.names {
            // A fieldless enum's, its names given before where it is.
            Some(names) => {
                let names = Expr::array(names.into_iter().map(Expr::str).collect());
                let [value, place, none] = ["value", "where", "none"].map(|word| js::Name::declared(word.into()));
                let call = Expr::call(
                    Expr::helper(check.helper),
                    vec![Expr::var(&value), names, Expr::var(&place), Expr::var(&none)],
                );
                Expr::arrow(
                    vec![value.into(), place.into(), none.into()],
                    vec![js::StmtKind::Return(Some(call)).at(js::Span::NONE)],
                )
            }
            None => Expr::helper(check.helper),
        })
    }

    /// The type `did`'s checker's name in the module, `checkShlex` of its
    /// name, allotted the first time one of the module's checks asks, as the
    /// namer gives every name it makes: apart from each word the module
    /// takes, its functions', imports' and globals', and its other
    /// checkers', `checkShlex$1` of another type of that name, shlex's
    /// `bytes::Shlex` beside its root's (`names::fresh_apart`, ADR 1150).
    /// Each type is checked by its own checker, never another's fields.
    fn checker_name(&self, did: DefId) -> js::Name {
        let mut all = self.krate.checker_names.borrow_mut();
        let of = all.entry(self.module).or_default();
        let word = match of.get(&did) {
            Some(word) => word.clone(),
            None => {
                let mut taken: IndexSet<String> = (self.module_names.iter().cloned())
                    .chain(of.values().cloned())
                    .collect();
                let word =
                    rust_js_program::names::fresh_apart(&mut taken, &format!("check{}", self.tcx.item_name(did)));
                of.insert(did, word.clone());
                word
            }
        };
        js::Name::module(&word)
    }

    /// The crate's enum `did`'s checker, `checkShape`, its module's function,
    /// made once: its tag, then the fields of the variant it names. Of a tagged
    /// enum, a fieldless variant its name's string, one with fields an object
    /// of its `TAG`; of one of a tag of its own, `#[rust_js::tag]`, each an
    /// object of that key (ADRs 0033, 0284). Not an untagged one, one of an
    /// `otherwise` variant, of `#[repr]` numbers or of `bool`s: not yet.
    fn enum_checker(&self, did: DefId) -> Option<Expr> {
        let tcx = self.tcx;
        let type_params = self.type_params(did)?;
        let adt = tcx.adt_def(did);
        let declared = crate::bindings::declared_tag(tcx, did);
        // An untagged enum, or one of a variant that's its payload, an
        // untagged variant's or an `otherwise`'s (ADRs 0214, 0284): not yet.
        if crate::bindings::is_untagged(tcx, did)
            || (adt.variants().iter()).any(|variant| {
                self.variant_is_payload(adt, variant) || crate::bindings::is_untagged(tcx, variant.def_id)
            })
            || adt.repr().int.is_some()
            || (adt.variants().iter()).any(|v| {
                crate::bindings::given_bool(tcx, v.def_id).is_some()
                    || crate::bindings::is_tagged_otherwise(tcx, did, v)
            })
        {
            return None;
        }
        let name = self.checker_name(did);
        let reference = Expr::var(&name);
        if self.dependencies.borrow().checkers.contains_key(&did) {
            return Some(reference);
        }
        self.dependencies.borrow_mut().checkers.insert(did, None);
        let key = crate::bindings::tag_key(tcx, did);
        let args = ty::GenericArgs::identity_for_item(tcx, did);
        self.dependencies.borrow_mut().checker_params.push(type_params.clone());
        let (mut objects, mut names, mut variants) = (Vec::new(), Vec::new(), Vec::new());
        for variant in adt.variants() {
            let tag = crate::bindings::variant_name(tcx, variant);
            let fields = self.variant_fields(variant, args);
            match fields.is_empty() && declared.is_none() {
                true => names.push(tag.clone()),
                false => objects.push(tag.clone()),
            }
            let mut checks = Vec::new();
            for (field, field_ty) in fields {
                let (check, none) = match self.contract(field_ty) {
                    Contract::Option(inner) if !self.boxed_payload(inner) => (self.checker(inner), Some("nullish")),
                    _ => (self.checker(field_ty), None),
                };
                if let Some(check) = check {
                    checks.push((field, check, none));
                }
            }
            variants.push((tag, checks));
        }
        self.dependencies.borrow_mut().checker_params.pop();
        let value = crate::temporary_names::declared(Role::Value);
        let params = std::iter::once(value.clone()).chain(type_params.into_iter().map(|(_, name)| name));
        let function = js::Function {
            name,
            params: params.map(Into::into).collect(),
            body: tagged_body(&value, &key, (objects, names), variants),
            export: false,
            is_async: false,
            noinline: false,
            span: js::Span::NONE,
            name_span: js::Span::NONE,
            param_spans: vec![js::Span::NONE],
            set_apart: false,
        };
        (self.dependencies.borrow_mut().checkers).insert(did, Some(crate::Checker::Function(function)));
        Some(reference)
    }

    /// The checks of `ty`'s `fields`, a struct's, on its object, but of a
    /// key in `skip`, one a struct it's flattened into names itself: each
    /// field's key, its check and its `None`, a flattened one's in turn.
    fn field_checks(
        &self,
        (ty, fields): (Ty<'tcx>, Vec<(crate::bindings::FieldKey, Ty<'tcx>)>),
        skip: &[String],
        out: &mut Vec<FieldCheck>,
    ) {
        // Its own keys, which shadow a flattened field's of one name.
        let own: Vec<String> = (fields.iter().enumerate())
            .filter(|&(at, _)| !crate::bindings::is_flatten_field(self.tcx, ty, at))
            .filter_map(|(_, (key, _))| key.name().map(str::to_owned))
            .collect();
        for (at, (key, field)) in fields.into_iter().enumerate() {
            // A `#[rust_js::flatten]` field's keys are the struct's
            // own (ADR 0204): its fields checked on the struct's object,
            // its failure the struct's path, but one the struct names
            // itself, whose key is the struct's, its value lost.
            if crate::bindings::is_flatten_field(self.tcx, ty, at) {
                if let Shape::Object(inner) = self.shape(field.peel_refs()) {
                    let mut skip: Vec<String> = skip.to_vec();
                    skip.extend(own.iter().cloned());
                    self.field_checks((field.peel_refs(), inner), &skip, out);
                }
                continue;
            }
            if key
                .name()
                .is_some_and(|name| skip.iter().any(|skipped| skipped == name))
            {
                continue;
            }
            // Its key: its name, or the symbol that names it, read
            // where the checker is (ADR 1051).
            let key = match key.symbol_item() {
                Some((item, span)) => Err(self.field_symbol(item, span)),
                None => Ok(key.text().to_owned()),
            };
            let (check, none) = match self.contract(field) {
                Contract::Option(inner) if !self.boxed_payload(inner) => (self.checker(inner), Some("nullish")),
                _ => (self.checker(field), None),
            };
            let Some(check) = check else { continue };
            // A props field with a default, which a JS caller may leave
            // out for the component it's passed to (ADR 0212): its
            // `undefined` too.
            let defaulted =
                (self.struct_field_def(ty, at)).is_some_and(|def| crate::bindings::has_field_default(self.tcx, def));
            let none = match (none, defaulted) {
                (None, true) => Some("undefined"),
                (none, _) => none,
            };
            out.push((key, check, none));
        }
    }

    /// Whether a value of `ty` is itself as a JS `Map` or `Set` compares it,
    /// `SameValueZero`: a number, a string, a `bool` or a `char`.
    fn plain_key(&self, ty: Ty<'tcx>) -> bool {
        match self.contract(ty) {
            Contract::Num(num) => !num.big(),
            Contract::Str | Contract::Bool | Contract::Char => true,
            Contract::Shared(inner) => self.plain_key(inner),
            _ => false,
        }
    }

    /// A generic type's type parameters, each with the name of the checker
    /// its checker is given, `T`, by its index; none of one of a `const`
    /// parameter, not checked yet.
    fn type_params(&self, did: DefId) -> Option<Vec<(u32, js::Name)>> {
        let mut params = Vec::new();
        for param in &self.tcx.generics_of(did).own_params {
            match param.kind {
                ty::GenericParamDefKind::Lifetime => {}
                ty::GenericParamDefKind::Type { .. } => {
                    params.push((param.index, js::Name::declared(param.name.to_string())))
                }
                ty::GenericParamDefKind::Const { .. } => return None,
            }
        }
        Some(params)
    }

    /// The checker of a generic type's use of type arguments `args`, its
    /// checker given each's: one checker of each type, whatever its uses,
    /// `(value) => checkPair(value, $checkU32)`; none where one isn't checked.
    fn checker_of_use(&self, reference: Expr, args: ty::GenericArgsRef<'tcx>) -> Option<Expr> {
        let types: Vec<Ty<'tcx>> = args.types().collect();
        if types.is_empty() {
            return Some(reference);
        }
        let checkers = (types.into_iter())
            .map(|t| self.checker(t))
            .collect::<Option<Vec<_>>>()?;
        let value = declared(Role::Value);
        let call = Expr::call(reference, std::iter::once(Expr::var(&value)).chain(checkers).collect());
        Some(Expr::arrow(
            vec![value.into()],
            vec![js::StmtKind::Return(Some(call)).at(js::Span::NONE)],
        ))
    }

    /// The crate's struct `did`'s checker, `checkOrder`, its module's
    /// function, made once; none where the crate has a function of that
    /// name, which a module may import, or the struct isn't an object of named fields.
    fn struct_checker(&self, did: DefId) -> Option<Expr> {
        let type_params = self.type_params(did)?;
        let ty = self.tcx.type_of(did).instantiate_identity().skip_normalization();
        // A library's too, of its name, apart from the crate's own of it,
        // `checkManifest$1`, as the namer gives it.
        let name = self.checker_name(did);
        let reference = Expr::var(&name);
        // Made, or being made: a struct that holds itself checks by its own.
        if self.dependencies.borrow().checkers.contains_key(&did) {
            return Some(reference);
        }
        self.dependencies.borrow_mut().checkers.insert(did, None);
        let value = declared(Role::Value);
        let statement = |helper: &'static str, args: Vec<Expr>| {
            js::StmtKind::Expr(Expr::call(Expr::helper(helper), args)).at(js::Span::NONE)
        };
        self.dependencies.borrow_mut().checker_params.push(type_params.clone());
        let body = match self.shape(ty) {
            Shape::Object(fields) => {
                let mut checks = Vec::new();
                self.field_checks((ty, fields), &[], &mut checks);
                // Of its fields alone, each by its name: its table (#2403,
                // the lead's (B)), a constant its module makes once.
                if type_params.is_empty() && checks.iter().all(|(key, ..)| key.is_ok()) {
                    let brand = self.brand_key(ty).map(|key| {
                        let name = super::brands::with_article(self.tcx.item_name(did).as_str());
                        (key, name)
                    });
                    self.dependencies.borrow_mut().checker_params.pop();
                    let shape = shape_of(name, checks, brand);
                    (self.dependencies.borrow_mut().checkers).insert(did, Some(crate::Checker::Shape(shape)));
                    return Some(reference);
                }
                let mut body = vec![statement("$checkObject", vec![Expr::var(&value)])];
                for (key, check, none) in checks {
                    let key = match key {
                        Ok(text) => Expr::str(text),
                        Err(symbol) => symbol,
                    };
                    let mut args = vec![Expr::var(&value), key, check];
                    args.extend(none.map(Expr::str));
                    body.push(statement("$checkField", args));
                }
                body
            }
            // A tuple struct, an array of its fields, checked as a tuple is,
            // by its own checker, as one may hold itself.
            Shape::Array(items) => {
                let checkers = (items.iter())
                    .map(|&item| self.checker(item))
                    .collect::<Option<Vec<_>>>();
                let Some(checkers) = checkers else {
                    self.dependencies.borrow_mut().checkers.shift_remove(&did);
                    return None;
                };
                let args = vec![
                    Expr::var(&value),
                    Expr::js_undefined(),
                    Expr::js_undefined(),
                    Expr::array(checkers),
                ];
                vec![statement("$checkTuple", args)]
            }
            Shape::Other => {
                self.dependencies.borrow_mut().checker_params.pop();
                self.dependencies.borrow_mut().checkers.shift_remove(&did);
                return None;
            }
        };
        // A type no other crate can make: one Rust made, its brand's, first,
        // then its fields, which JS may still write (#2403 step 2c, ADR 1196).
        let mut body = body;
        if let Some(key) = self.brand_key(ty) {
            let name = super::brands::with_article(self.tcx.item_name(did).as_str());
            let undefined = Expr::js_undefined;
            let args = vec![
                Expr::var(&value),
                undefined(),
                undefined(),
                Expr::str(key),
                Expr::str(name),
            ];
            body.insert(0, statement("$checkBrand", args));
        }
        self.dependencies.borrow_mut().checker_params.pop();
        let params = std::iter::once(value).chain(type_params.into_iter().map(|(_, name)| name));
        let function = js::Function {
            name,
            params: params.map(Into::into).collect(),
            body,
            export: false,
            is_async: false,
            noinline: false,
            span: js::Span::NONE,
            name_span: js::Span::NONE,
            param_spans: vec![js::Span::NONE],
            set_apart: false,
        };
        (self.dependencies.borrow_mut().checkers).insert(did, Some(crate::Checker::Function(function)));
        Some(reference)
    }
}

/// A field's check: its key, its name or the symbol that names it; its
/// check; and the `None` it allows.
type FieldCheck = (Result<String, Expr>, Expr, Option<&'static str>);

/// A struct's checker `name` as its table, `checks` each a field's by its
/// name, and of a sealed type its brand's key and its name, "an Email":
/// `const checkPoint = /* @__PURE__ */ $checkShape(() => ({ x: $checkU32 }))`.
fn shape_of(name: js::Name, checks: Vec<FieldCheck>, brand: Option<(String, String)>) -> js::Const {
    let props = (checks.into_iter())
        .filter_map(|(key, check, none)| {
            let check = match none {
                Some(none) => Expr::array(vec![check, Expr::str(none)]),
                None => check,
            };
            Some(js::Prop::Field(key.ok()?, check))
        })
        .collect();
    let table = Expr::arrow(
        Vec::new(),
        vec![js::StmtKind::Return(Some(Expr::object(props))).at(js::Span::NONE)],
    );
    let mut args = vec![table];
    if let Some((key, name)) = brand {
        args.push(Expr::str(key));
        args.push(Expr::str(name));
    }
    let made = Expr::call(Expr::helper("$checkShape"), args);
    js::Const {
        name,
        value: Expr {
            facts: js::Facts::of(js::Known::PureCall),
            ..made
        },
        export: false,
        mutable: false,
        fields: js::Fields::default(),
        span: js::Span::NONE,
    }
}

/// What checks a value of a tagged enum's: `$checkTag`, which gives its
/// tag, `objects` each a variant's object's and `names` each a fieldless
/// one's string; then each variant's fields, `(key, checker, none)`.
type Variants = Vec<(String, Vec<(String, Expr, Option<&'static str>)>)>;

fn tagged_body(
    value: &js::Name,
    key: &str,
    (objects, names): (Vec<String>, Vec<String>),
    variants: Variants,
) -> Vec<js::Stmt> {
    let strings = |items: Vec<String>| Expr::array(items.into_iter().map(Expr::str).collect());
    let tag = declared(Role::Tag);
    let read = Expr::call(
        Expr::helper("$checkTag"),
        vec![Expr::var(value), Expr::str(key), strings(objects), strings(names)],
    );
    let mut body = vec![js::StmtKind::Const(tag.clone(), read).at(js::Span::NONE)];
    for (variant, fields) in variants.into_iter().filter(|(_, fields)| !fields.is_empty()) {
        let checks = (fields.into_iter())
            .map(|(field, checker, none)| {
                let mut args = vec![Expr::var(value), Expr::str(field), checker];
                args.extend(none.map(Expr::str));
                js::StmtKind::Expr(Expr::call(Expr::helper("$checkField"), args)).at(js::Span::NONE)
            })
            .collect();
        let test = Expr::bin(js::Op::Eq, Expr::var(&tag), Expr::str(variant));
        body.push(js::StmtKind::If(test, checks, None).at(js::Span::NONE));
    }
    body
}

/// `(value, where, none) => $checkArray(value, where, none, item)`, of
/// `length` items where it's an array's.
fn array_checker(item: Expr, length: Option<u64>) -> Expr {
    let mut args = vec![item];
    if let Some(length) = length {
        args.push(Expr::num(length as f64));
    }
    arrow_of("$checkArray", args)
}

/// `(value, where, none) => helper(value, where, none, ..args)`.
fn arrow_of(helper: &'static str, args: Vec<Expr>) -> Expr {
    let [value, place, none] = ["value", "where", "none"].map(|word| js::Name::declared(word.into()));
    let mut all = vec![Expr::var(&value), Expr::var(&place), Expr::var(&none)];
    all.extend(args);
    Expr::arrow(
        vec![value.into(), place.into(), none.into()],
        vec![js::StmtKind::Return(Some(Expr::call(Expr::helper(helper), all))).at(js::Span::NONE)],
    )
}
