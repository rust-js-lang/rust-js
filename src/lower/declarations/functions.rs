//! What a function is to its declaration: `export function Tag(props:
//! TagProps): ReactNode;`, its parameters, which of them may be left out,
//! and what it gives; and the names of a function type's parameters, as
//! the Rust names them (#543).

use std::collections::HashMap;

use rustc_hir as hir;
use rustc_hir::attrs::lang_items::LangItem;
use rustc_middle::ty::{self, Ty, TyCtxt};
use rustc_span::Ident;
use rustc_span::def_id::DefId;
use serde_json::{Value, json};

use super::{Declarations, documented, keyword};

/// Where a function's parameters, or a function type's, may start being
/// left out: its trailing `Option`s, before a rest parameter `rest` too,
/// but one `nullable`, which is `T | null` and given (ADRs 0443, 0664,
/// 0861). The one rule of both, as a caller leaves a trailing `None` out
/// of each (ADR 0313, #544).
pub(super) fn optional_from<'tcx>(
    tcx: TyCtxt<'tcx>,
    inputs: &[Ty<'tcx>],
    rest: Option<usize>,
    nullable: impl Fn(usize) -> bool,
) -> usize {
    let is_option =
        |ty: Ty<'tcx>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option));
    let before = &inputs[..rest.unwrap_or(inputs.len())];
    before.len()
        - (before.iter().enumerate().rev())
            .take_while(|&(i, &ty)| is_option(ty) && !nullable(i))
            .count()
}

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
    /// A parameter's name, `ident`, `function`'s `i`th of type `ty`, as its
    /// JS spells it, a word JS keeps given apart, `new$`; its JS name's,
    /// without a `_` of one unused (ADR 0678). Its `self`, named after its
    /// type, as its body names it, `tick(counter: Counter)` (#507).
    fn parameter_js_name(&self, function: DefId, ident: Ident, ty: Option<Ty<'tcx>>) -> String {
        let receiver = (ident.name == rustc_span::kw::SelfLower)
            .then(|| ty.and_then(|ty| super::super::analysis::receiver_name(self.tcx, ty)))
            .flatten();
        let name = receiver.unwrap_or_else(|| ident.name.to_string());
        let named = super::super::parameter_name(self.tcx, function, &name);
        crate::names::js_ident(&named)
    }

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
                Some(self.parameter_js_name(function, ident, inputs.map(|inputs| inputs[i])))
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

    /// What `function`'s JS takes after its own parameters, as its
    /// declaration declares it, in `evidence_slots`' order, of the same
    /// owners, `takes_location`'s, `const_params`', `facts_asked`' and
    /// `bounds`': its caller's location, `location: { readonly file: string;
    /// .. }`, and each const parameter's value, `N: number` (#560); each
    /// bound's dictionary of a trait that's declared, `SShape: Shape<S>`
    /// (#550), named as its JS names it (`bound_word`). Or what a JS caller
    /// can't give, the first (#549, the user's decision, 2026-11-03, ADR
    /// 1077): a fact of a type parameter, `size(TSize)` (ADR 0145), which
    /// only a Rust caller knows; or a bound's dictionary of another trait,
    /// `shown(value, TDisplay)`, a std trait's the runtime's own. (A rest
    /// parameter is given none, ADR 0664.) What a JS caller leaves out, a
    /// drop or a copy's, isn't asked.
    pub(super) fn evidence(&mut self, function: DefId) -> Result<Vec<Value>, String> {
        let generics = self.tcx.generics_of(function);
        if let Some(&(index, fact)) = self.type_facts.get(&function).and_then(|facts| facts.first()) {
            let of = match (index as usize) < generics.count() {
                true => format!("`{}`'s", generics.param_at(index as usize, self.tcx).name),
                false => "an associated type's".to_string(),
            };
            return Err(format!("{of} {}", fact.described()));
        }
        let mut taken: std::collections::HashSet<String> =
            self.parameter_names(function).into_iter().flatten().collect();
        let bounds = super::super::traits::bounds(self.tcx, self.foreign, self.bounds_given, function);
        if let Some(bound) = bounds.iter().find(|bound| !self.declares_dictionary(bound.def_id)) {
            let (of, name) = (bound.self_ty(), self.tcx.item_name(bound.def_id));
            return Err(format!("`{of}`'s `{name}` dictionary"));
        }
        let mut params = Vec::new();
        // Its caller's location, which a JS caller gives as a Rust caller
        // does, where it's called from (ADR 0427, #560).
        if super::super::std_types::location::takes_location(self.caller_locations, self.foreign, function) {
            let name = crate::names::fresh_in(&mut taken, "location");
            let ty = self.ts(self.tcx.caller_location_ty());
            params.push(json!({ "name": name, "optional": false, "rest": false, "type": ty }));
        }
        // Each const parameter's value, `N`, a number a JS caller gives as
        // it's typed (ADR 0107, #560).
        for param in super::super::traits::const_params(self.tcx, function) {
            let name = crate::names::fresh_in(&mut taken, param.name.as_str());
            let ty = self
                .tcx
                .type_of(param.def_id)
                .instantiate_identity()
                .skip_normalization();
            let ty = self.ts(ty);
            params.push(json!({ "name": name, "optional": false, "rest": false, "type": ty }));
        }
        for bound in bounds {
            let ty = self.dictionary_type(bound);
            let name = crate::names::fresh_in(&mut taken, &super::super::traits::bound_word(self.tcx, bound));
            params.push(json!({ "name": name, "optional": false, "rest": false, "type": ty }));
        }
        Ok(params)
    }

    /// Whether `function`, which its declaration names `name`, `size` or a
    /// member's `Counter.sizeOf`, is left out (`evidence`): then a note is
    /// where it would be, what it takes and that a JS caller can't give it,
    /// after the declaration it's of (`notes`, #549, #559).
    pub(super) fn leave_out(&mut self, function: DefId, name: &str) -> bool {
        let Err(ungiven) = self.evidence(function) else {
            return false;
        };
        self.notes.push(json!({
            "kind": "note",
            "text": format!("// `{name}` isn't declared: its JS takes {ungiven} after its arguments, which a JS caller can't give (ADR 1077)."),
        }));
        true
    }

    /// `function_signature`'s, in `def_id`'s own context: what a `&mut` or
    /// a `dyn` in it is is its body's (`Representation`, #619, #620).
    pub(super) fn function(&mut self, def_id: DefId) -> Value {
        let outer = std::mem::replace(&mut self.owner, def_id);
        let function = self.function_signature(def_id);
        self.owner = outer;
        function
    }

    fn function_signature(&mut self, def_id: DefId) -> Value {
        let sig = self
            .tcx
            .fn_sig(def_id)
            .instantiate_identity()
            .skip_normalization()
            .skip_binder();
        let idents = self.tcx.fn_arg_idents(def_id);
        let decl = (def_id.as_local()).and_then(|local| self.tcx.hir_node_by_def_id(local).fn_decl());
        // Its trailing `Option`s are optional, `greeting?: string | null`, so a
        // JS caller leaves them out (ADR 0443).
        let tcx = self.tcx;
        let is_option =
            |ty: Ty<'tcx>| matches!(ty.kind(), ty::Adt(adt, _) if tcx.is_lang_item(adt.did(), LangItem::Option));
        // Its last, a slice marked `#[rust_js::rest]`, is a rest parameter,
        // `...args: T[]`, which those before it may still be left out before
        // (ADR 0664).
        let rest = super::super::bindings::takes_rest(self.tcx, def_id).then(|| sig.inputs().len() - 1);
        // One it names `#[rust_js::nullable(..)]` is `T | null` as declared,
        // never left out, which each caller gives `null` for `None` and the
        // body takes as it is (ADR 0861, amended, the user's decision,
        // 2026-10-27).
        let nullable_params = super::super::bindings::nullable_params(self.tcx, def_id);
        let nullable = |i: usize| {
            (idents.get(i).copied().flatten()).is_some_and(|ident| nullable_params.contains(&ident.name))
                && is_option(sig.inputs()[i])
        };
        // What its JS takes after them, of a function a JS caller calls,
        // not a trait's method's, which its dictionary's type declares, nor a
        // binding's (`evidence`); after which none may be left out.
        let own = self.tcx.def_kind(def_id) == rustc_hir::def::DefKind::Fn
            || (self.tcx.trait_of_assoc(def_id).is_none() && self.tcx.trait_item_of(def_id).is_none());
        let evidence = match own && !super::super::bindings::is_binding(self.tcx, def_id) {
            true => self.evidence(def_id).unwrap_or_default(),
            false => Vec::new(),
        };
        let optional_from = match evidence.is_empty() {
            true => optional_from(tcx, sig.inputs(), rest, nullable),
            false => sig.inputs().len(),
        };
        let mut params: Vec<Value> = (sig.inputs().iter().enumerate())
            .map(|(i, &ty)| {
                // As its JS spells it, a word JS keeps given apart, `new$`.
                let name = match idents.get(i).copied().flatten() {
                    Some(ident) => self.parameter_js_name(def_id, ident, Some(ty)),
                    // A pattern, `TagProps { variant, .. }`: its props'.
                    None => "props".to_string(),
                };
                let written = decl.and_then(|decl| decl.inputs.get(i));
                let ty = match nullable(i).then(|| self.option(ty)).flatten() {
                    Some(inner) => {
                        let inner = self.as_written(written.and_then(|w| self.written_option(w)), |out| out.ts(inner));
                        json!({ "kind": "union", "types": [inner, keyword("null")] })
                    }
                    None => self.as_written(written, |out| out.ts(ty)),
                };
                let rest = rest == Some(i);
                json!({ "name": name, "optional": i >= optional_from && !rest, "rest": rest, "type": ty })
            })
            .collect();
        params.extend(evidence);
        let output = sig.output();
        let written = decl.and_then(|decl| match decl.output {
            hir::FnRetTy::Return(written) => Some(written),
            hir::FnRetTy::DefaultReturn(_) => None,
        });
        // A nullable result's `None` is `null` where it's made, never
        // `undefined`, `string | null` (ADRs 0574, 0578).
        let nullable = (super::super::bindings::nullable_result(self.tcx, def_id))
            .then(|| self.option(output))
            .flatten();
        let returns = match (output.is_unit(), nullable) {
            (true, _) => keyword("void"),
            (false, Some(inner)) => {
                let inner = self.as_written(written.and_then(|w| self.written_option(w)), |out| out.ts(inner));
                json!({ "kind": "union", "types": [inner, keyword("null")] })
            }
            (false, None) => self.as_written(written, |out| out.ts(output)),
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
                let mut params = function["params"].as_array().cloned().unwrap_or_default();
                if !params.is_empty() {
                    params.remove(0);
                }
                let declared = match getter {
                    true => json!({ "kind": "getter", "name": name, "returns": function["returns"] }),
                    false => json!({
                        "kind": "method",
                        "name": name,
                        "static": false,
                        "typeParameters": function["typeParameters"],
                        "params": params,
                        "returns": function["returns"],
                    }),
                };
                members.push(documented(declared, self.tcx, declaring));
            }
        }
        members
    }
}
