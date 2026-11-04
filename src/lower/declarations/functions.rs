//! What a function is to its declaration: `export function Tag(props:
//! TagProps): ReactNode;`, its parameters, which of them may be left out,
//! and what it gives; and the names of a function type's parameters, as
//! the Rust names them (#543).

use std::collections::HashMap;

use rustc_hir as hir;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::def_id::DefId;
use serde_json::{Value, json};

use super::super::signatures::{Form, JsParam, Origin, js_parameter_name, optional_from};
use super::{Declarations, documented, keyword};

/// The first function type in `ty`, a fn pointer or a `dyn Fn`: the one
/// a field of `Rc<dyn Fn(..)>` or `Option<fn(..)>` holds.
fn first_function_type<'tcx>(tcx: TyCtxt<'tcx>, ty: Ty<'tcx>) -> Option<Ty<'tcx>> {
    ty.walk().filter_map(|arg| arg.as_type()).find(|ty| match ty.kind() {
        ty::FnPtr(..) => true,
        ty::Dynamic(traits, ..) => {
            (traits.principal_def_id()).is_some_and(|t| tcx.fn_trait_kind_from_def_id(t).is_some())
        }
        _ => false,
    })
}

/// What fills a field, of the type of the value its literal gives,
/// before it's made the field's: the closure or the function in it,
/// `Rc<{closure}>`'s, `Some(Box<fn item>)`'s.
fn filler(ty: Ty<'_>) -> Option<DefId> {
    ty.walk()
        .filter_map(|arg| arg.as_type())
        .find_map(|ty| match ty.kind() {
            ty::Closure(def_id, _) | ty::FnDef(def_id, _) => Some(*def_id),
            _ => None,
        })
}

/// What a literal, `expr`, fills each of its function-typed fields with,
/// into `fillers` by the field: `None` where it's no closure or function.
fn literal_fillers<'tcx>(
    tcx: TyCtxt<'tcx>,
    typeck: &'tcx ty::TypeckResults<'tcx>,
    expr: &'tcx hir::Expr<'tcx>,
    fillers: &mut HashMap<DefId, Vec<Option<DefId>>>,
) {
    if let hir::ExprKind::Struct(qpath, fields, _) = expr.kind
        && let ty::Adt(adt, _) = typeck.expr_ty(expr).kind()
    {
        let variant = adt.variant_of_res(typeck.qpath_res(qpath, expr.hir_id));
        for field in fields {
            let def = &variant.fields[typeck.field_index(field.hir_id)];
            let declared = tcx.type_of(def.did).instantiate_identity().skip_normalization();
            if def.did.is_local() && first_function_type(tcx, declared).is_some() {
                // A block's, its value's, which is made the field's
                // where the block ends: `{ let s = ..; Rc::new(|..| ..) }`.
                let mut value = field.expr;
                while let hir::ExprKind::Block(block, _) = value.kind
                    && let Some(tail) = block.expr
                {
                    value = tail;
                }
                let given = filler(typeck.expr_ty(value));
                fillers.entry(def.did).or_default().push(given);
            }
        }
    }
}

/// The functions and closures a static's or a constant's value, `item`'s,
/// may be, by what each branch of it gives, `None` of one that's another
/// value: its initializer's, a thread-local's `init` (ADR 0037), through
/// its blocks, its `if`s' and `match`es' branches, and a `&` of one.
fn given_values(tcx: TyCtxt<'_>, item: DefId) -> Vec<Option<DefId>> {
    let Some(local) = item.as_local() else {
        return Vec::new();
    };
    // A thread-local's value is its `init` function's.
    let init = (super::super::analysis::is_thread_local(tcx, local))
        .then(|| {
            (tcx.hir_crate_items(()).definitions()).find(|&d| {
                tcx.def_kind(d) == rustc_hir::def::DefKind::Fn && super::super::analysis::init_of(tcx, d) == Some(local)
            })
        })
        .flatten();
    let owner = init.unwrap_or(local);
    let Some(body) = tcx.hir_maybe_body_owned_by(owner) else {
        return Vec::new();
    };
    let typeck = tcx.typeck(owner);
    let mut given = Vec::new();
    let mut values = vec![body.value];
    while let Some(value) = values.pop() {
        match value.kind {
            hir::ExprKind::Block(block, _) => values.extend(block.expr),
            hir::ExprKind::If(_, then, otherwise) => values.extend(std::iter::once(then).chain(otherwise)),
            hir::ExprKind::Match(_, arms, _) => values.extend(arms.iter().map(|arm| arm.body)),
            hir::ExprKind::AddrOf(_, _, inner) | hir::ExprKind::DropTemps(inner) => values.push(inner),
            _ => given.push(filler(typeck.expr_ty(value))),
        }
    }
    given
}

impl<'tcx> Declarations<'_, 'tcx> {
    /// Each of `function`'s parameters, a fn's or a closure's, by its name
    /// as `function`'s declaration names it; `None` of a pattern or a `_`,
    /// which names none.
    fn parameter_names(&self, function: DefId) -> Vec<Option<String>> {
        let inputs = match self.tcx.is_closure_like(function) {
            true => None,
            false => Some(
                self.tcx
                    .fn_sig(function)
                    .instantiate_identity()
                    .skip_normalization()
                    .skip_binder()
                    .inputs(),
            ),
        };
        (self.tcx.fn_arg_idents(function).iter().enumerate())
            .map(|(i, ident)| {
                let ident = ident.filter(|ident| ident.name != rustc_span::kw::Underscore)?;
                Some(js_parameter_name(
                    self.tcx,
                    function,
                    ident,
                    inputs.map(|inputs| inputs[i]),
                ))
            })
            .collect()
    }

    /// `(event: MouseEvent<Element>) => void`: a function type, each
    /// parameter named as the Rust names it, `named` (#543), else by its
    /// type, as a person names one.
    /// Its last parameter of rest arguments as a value, `&dyn js::Rest<T>`,
    /// a rest parameter of what each reads as, `...args: unknown[]` (ADR
    /// 0851). Its trailing `Option`s are optional, `sub?: Node | null`, as
    /// a function's are (`optional_from`), where a caller of what may
    /// leave them out does (ADR 0313, #544): not a JS class's constructor.
    pub(super) fn function_type(
        &mut self,
        inputs: &[Ty<'tcx>],
        output: Ty<'tcx>,
        named: &[Option<String>],
        may_leave_out: bool,
    ) -> Value {
        let count = inputs.len();
        let rest = (inputs.last())
            .and_then(|&last| super::super::bindings::rest_items(self.tcx, last))
            .map(|items| (count - 1, items));
        let optional_from = match may_leave_out {
            true => optional_from(self.tcx, inputs, rest.map(|(at, _)| at), |_| false),
            false => count,
        };
        let mut names: Vec<String> = named.iter().flatten().cloned().collect();
        let params: Vec<Value> = (inputs.iter().enumerate())
            .map(|(i, &input)| {
                let given = named.get(i).cloned().flatten();
                if let Some((at, items)) = rest
                    && at == i
                {
                    let name = given.unwrap_or_else(|| "args".to_string());
                    return json!({ "name": name, "optional": false, "rest": true, "type": self.ts(items) });
                }
                let ty = self.ts(input);
                let name = given.unwrap_or_else(|| {
                    let base = super::param_name(&ty);
                    let taken = names
                        .iter()
                        .filter(|n| n.trim_end_matches(char::is_numeric) == base)
                        .count();
                    let name = if taken == 0 {
                        base
                    } else {
                        format!("{base}{}", taken + 1)
                    };
                    names.push(name.clone());
                    name
                });
                json!({ "name": name, "optional": i >= optional_from, "rest": false, "type": ty })
            })
            .collect();
        let returns = match output.is_unit() {
            true => keyword("void"),
            false => self.ts(output),
        };
        json!({ "kind": "function", "typeParameters": [], "params": params, "returns": returns })
    }

    /// A function type's parameters' names, of those in the type being
    /// declared (`function_names`): the last pushed of `ty`'s, as the
    /// innermost written wins, none where nothing names them.
    pub(super) fn function_names(&self, ty: Ty<'tcx>) -> Vec<Option<String>> {
        (self.function_names.iter().rev())
            .find(|(named, _)| *named == ty)
            .map(|(_, names)| names.clone())
            .unwrap_or_default()
    }

    /// The names of the parameters of the function type a field `field`
    /// of type `ty` holds, as the crate fills it (`filled_names`), while
    /// its type is declared, until `function_names` is truncated.
    pub(super) fn fill_names(&mut self, field: DefId, ty: Ty<'tcx>) {
        let Some(function) = first_function_type(self.tcx, ty) else {
            return;
        };
        let names = self.filled_names(field);
        if names.iter().any(Option::is_some) {
            let function = self.tcx.erase_and_anonymize_regions(function);
            self.function_names.push((function, names));
        }
    }

    /// The names a `dyn Fn` field's parameters have where every literal of
    /// its struct or variant the crate writes fills it with a closure or a
    /// function that names each alike, `link: Rc::new(move |dep, sub,
    /// version| ..)`'s `(dep, sub, version)`, as a person names a field's
    /// function type by the function it holds; `None` where one names it
    /// otherwise, or fills it with what names none (#543, ADR 0196,
    /// amended). A library's consumers' literals aren't its: the library's
    /// declarations are made of its own.
    fn filled_names(&mut self, field: DefId) -> Vec<Option<String>> {
        if self.filled.is_none() {
            let tcx = self.tcx;
            let mut fillers = HashMap::new();
            super::super::aggregates::each_struct_literal(tcx, |typeck, expr| {
                literal_fillers(tcx, typeck, expr, &mut fillers);
            });
            let filled = (fillers.into_iter())
                .map(|(field, given)| (field, self.agreed_names(&given)))
                .collect();
            self.filled = Some(filled);
        }
        (self.filled.as_ref())
            .and_then(|filled| filled.get(&field).cloned())
            .unwrap_or_default()
    }

    /// The names the functions and closures `given` name their parameters
    /// alike, each `None` where two name it otherwise: what's given that's
    /// no closure or function, a variable or a `js::cast`, says nothing of
    /// them (#543, #630).
    fn agreed_names(&self, given: &[Option<DefId>]) -> Vec<Option<String>> {
        let mut names: Option<Vec<Option<String>>> = None;
        for &given in given.iter().flatten() {
            let own = self.parameter_names(given);
            names = Some(match names {
                None => own,
                Some(names) => (names.into_iter().enumerate())
                    .map(|(i, name)| name.filter(|name| own.get(i).cloned().flatten().as_ref() == Some(name)))
                    .collect(),
            });
        }
        names.unwrap_or_default()
    }

    /// The names of the parameters of the function type a static's or a
    /// constant's value, `item`'s of type `ty`, holds, as the functions it
    /// may be name them (`given_names`), while its type is declared, until
    /// `function_names` is truncated: SchedulerNative's `pub static
    /// UNSTABLE_SCHEDULE_CALLBACK: &'static ScheduleCallback = if .. { ..
    /// } else { &scheduler::unstable_schedule_callback }`'s
    /// `(priorityLevel, callback, options)` (#630).
    /// The names pushed of before, which truncating to gives back.
    pub(super) fn fill_value_names(&mut self, item: DefId, ty: Ty<'tcx>) -> usize {
        let outer = self.function_names.len();
        let Some(function) = first_function_type(self.tcx, ty) else {
            return outer;
        };
        let names = self.agreed_names(&given_values(self.tcx, item));
        if names.iter().any(Option::is_some) {
            let function = self.tcx.erase_and_anonymize_regions(function);
            self.function_names.push((function, names));
        }
        outer
    }

    /// The names a fn pointer written in `written` gives its parameters,
    /// `fn(dep: &Node, sub: &Node)`'s, each by its type (`function_names`).
    pub(super) fn written_function_names(
        &self,
        written: &hir::Ty<'tcx>,
        named: &mut Vec<(Ty<'tcx>, Vec<Option<String>>)>,
    ) {
        match &written.kind {
            hir::TyKind::FnPtr(function) => {
                let names = (function.param_idents.iter())
                    .map(|ident| {
                        let ident = ident.filter(|ident| ident.name != rustc_span::kw::Underscore)?;
                        Some(super::super::written_parameter_name(ident.name.as_str()))
                    })
                    .collect::<Vec<_>>();
                if names.iter().any(Option::is_some) {
                    let ty = rustc_hir_analysis::lower_ty(self.tcx, written);
                    named.push((self.tcx.erase_and_anonymize_regions(ty), names));
                }
                for input in function.decl.inputs {
                    self.written_function_names(input, named);
                }
                if let hir::FnRetTy::Return(output) = function.decl.output {
                    self.written_function_names(output, named);
                }
            }
            hir::TyKind::Path(hir::QPath::Resolved(_, path)) => {
                for arg in path.segments.iter().filter_map(|s| s.args).flat_map(|args| args.args) {
                    if let hir::GenericArg::Type(ty) = arg {
                        self.written_function_names(ty.as_unambig_ty(), named);
                    }
                }
            }
            hir::TyKind::Ref(_, inner) | hir::TyKind::Ptr(inner) => self.written_function_names(inner.ty, named),
            hir::TyKind::Slice(inner) | hir::TyKind::Array(inner, _) => self.written_function_names(inner, named),
            hir::TyKind::Tup(items) => {
                for item in *items {
                    self.written_function_names(item, named);
                }
            }
            _ => {}
        }
    }

    /// Whether `function`, which its declaration names `name`, `size` or a
    /// member's `Counter.sizeOf`, is left out: its JS takes what a JS
    /// caller can't give after its arguments (`JsSignature::ungiven`), a
    /// fact of a type parameter, `size(TSize)` (ADR 0145), which only a Rust
    /// caller knows, or a bound's dictionary of a trait that isn't declared,
    /// `shown(value, TDisplay)`, a std trait's the runtime's own (#549, the
    /// user's decision, 2026-11-03, ADR 1077). Then a note is where it would
    /// be, what it takes and that a JS caller can't give it, after the
    /// declaration it's of (`notes`, #549, #559).
    pub(super) fn leave_out(&mut self, function: DefId, name: &str) -> bool {
        let signature = (self.representation.signature)(function);
        let Some(ungiven) = signature.ungiven(self.tcx, function, |t| self.declares_dictionary(t)) else {
            return false;
        };
        self.notes.push(json!({
            "kind": "note",
            "text": format!("// `{name}` isn't declared: its JS takes {ungiven} after its arguments, which a JS caller can't give (ADR 1077)."),
        }));
        true
    }

    /// `function_signature`'s, in `def_id`'s own context: what a `&mut` or
    /// a `dyn` in it is is its body's (`SignatureForms`, #619, #620).
    pub(super) fn function(&mut self, def_id: DefId) -> Value {
        let outer = std::mem::replace(&mut self.owner, def_id);
        let function = self.function_signature(def_id);
        self.owner = outer;
        function
    }

    /// `export function Tag(props: TagProps): ReactNode;`: what `def_id`'s
    /// JS takes and gives, its `JsSignature`, lowering's answer, each part
    /// typed as the Rust writes it, by the aliases it names (`as_written`).
    /// No declaration says on its own what a function takes.
    fn function_signature(&mut self, def_id: DefId) -> Value {
        let signature = (self.representation.signature)(def_id);
        let decl = (def_id.as_local()).and_then(|local| self.tcx.hir_node_by_def_id(local).fn_decl());
        // What a JS caller can't give, which leaves it out (`leave_out`):
        // nothing it takes after its arguments then.
        let given = (signature.ungiven(self.tcx, def_id, |t| self.declares_dictionary(t))).is_none();
        let params: Vec<Value> = (signature.params.iter())
            .filter(|param| given || matches!(param.origin, Origin::Rust(_)))
            .filter_map(|param| self.parameter(param, decl))
            .collect();
        let written = decl.and_then(|decl| match decl.output {
            hir::FnRetTy::Return(written) => Some(written),
            hir::FnRetTy::DefaultReturn(_) => None,
        });
        let returns = match self.form_ts(signature.result, written) {
            Some((returns, _)) => returns,
            None => keyword("void"),
        };
        json!({
            "kind": "function",
            "name": self.js_name(def_id),
            "exported": true,
            "declare": false,
            "typeParameters": self.generics(def_id),
            "params": params,
            "returns": returns,
        })
    }

    /// `name?: T`: one of what a `JsSignature` takes, typed as the Rust
    /// writes it, `decl`'s; `None` of one a JS caller doesn't give.
    fn parameter(&mut self, param: &JsParam<'tcx>, decl: Option<&'tcx hir::FnDecl<'tcx>>) -> Option<Value> {
        let written = match param.origin {
            Origin::Rust(i) => decl.and_then(|decl| decl.inputs.get(i)),
            Origin::Evidence(_) => None,
        };
        let (ty, optional) = self.form_ts(param.form, written)?;
        let rest = matches!(param.form, Form::Rest(_));
        Some(json!({ "name": param.name, "optional": optional, "rest": rest, "type": ty }))
    }

    /// What a value of `form` is to TypeScript, written `written`, and
    /// whether a caller may leave it out; `None` of what a JS caller
    /// doesn't give, and of a result that's nothing, `void`.
    fn form_ts(&mut self, form: Form<'tcx>, written: Option<&'tcx hir::Ty<'tcx>>) -> Option<(Value, bool)> {
        Some(match form {
            Form::Value { ty, optional } => (self.as_written(written, |out| out.ts(ty)), optional),
            Form::Rest(ty) => (self.as_written(written, |out| out.ts(ty)), false),
            // `T | null`: its `None` is `null`, as a caller gives it and the
            // body makes it (ADRs 0574, 0578, 0861).
            Form::Nullable(inner) => {
                let inner = self.as_written(written.and_then(|w| self.written_option(w)), |out| out.ts(inner));
                (json!({ "kind": "union", "types": [inner, keyword("null")] }), false)
            }
            Form::Dictionary(bound) => (self.dictionary_type(bound), false),
            Form::Unknowable | Form::LeftOut | Form::Void => return None,
        })
    }

    /// Each method of a trait of classes `class` implements, its
    /// prototype's, the impl's or the trait's default, as a class's
    /// method is declared (rd283, ADR 1035).
    pub(super) fn class_trait_methods(&mut self, class: DefId) -> Vec<Value> {
        let mut members = Vec::new();
        for imp in self.tcx.hir_crate_items(()).definitions() {
            let imp = imp.to_def_id();
            // Of the binding's class it extends, only what its impl writes:
            // the rest is that class's (rd331).
            let extending = crate::lower::classes::subclass_impl_class(self.tcx, imp) == Some(class);
            if crate::lower::classes::trait_impl_class(self.tcx, imp) != Some(class) && !extending {
                continue;
            }
            let trait_id = self.tcx.impl_trait_id(imp);
            if !self.tcx.visibility(trait_id).is_public() {
                continue;
            }
            // Of the class it extends, what its impl writes, in its order.
            let items: Vec<DefId> = match extending {
                true => (self.tcx.associated_item_def_ids(imp).iter())
                    .filter(|&&own| self.tcx.def_kind(own) == rustc_hir::def::DefKind::AssocFn)
                    .filter_map(|&own| self.tcx.trait_item_of(own))
                    .collect(),
                false => self.tcx.associated_item_def_ids(trait_id).to_vec(),
            };
            for item in items {
                // One its impl gives a field with is that field (rd304).
                let given = self.tcx.impl_item_implementor_ids(imp).get(&item).copied();
                if given.is_some_and(|own| crate::lower::classes::field_accessor(self.tcx, own).is_some()) {
                    continue;
                }
                // Of the class it extends, its impl's own, of its props' type and
                // its doc, not the trait's (rd331).
                let declaring = given.filter(|_| extending).unwrap_or(item);
                let function = self.function(declaring);
                let name = crate::lower::bindings::fn_name(self.tcx, item);
                // A getter, `get type(): string`, where the trait's is one (rd679).
                let getter = given.and_then(|own| crate::lower::classes::member(self.tcx, own))
                    == Some(crate::lower::classes::Member::Getter);
                let declared = match getter {
                    true => json!({ "kind": "getter", "name": name, "returns": function["returns"] }),
                    false => json!({
                        "kind": "method",
                        "name": name,
                        "static": false,
                        "typeParameters": function["typeParameters"],
                        "params": function["params"],
                        "returns": function["returns"],
                    }),
                };
                members.push(documented(declared, self.tcx, declaring));
            }
        }
        members
    }
}
