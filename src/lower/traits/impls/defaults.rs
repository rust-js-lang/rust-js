//! A trait's default body copied into an impl's dictionary (ADR 0495): its
//! method lowered in the impl's terms, given the impl's evidence and its
//! own. Split from `impls.rs` by concept (#1334).

use super::*;
use crate::lower::temporary_names::{Base, Role};

impl<'a, 'tcx> FnCx<'a, 'tcx> {
    pub(super) fn default_method(&mut self, imp: DefId, id: DefId, args: ty::GenericArgsRef<'tcx>) -> R<Expr> {
        let span = self.tcx.def_span(id);
        let args = default_args(self.tcx, imp, id, args);
        let mut specialized = Vec::new();
        // A generic default's own evidence is its caller's, after its
        // arguments (ADR 0106); the rest is the impl's, made here.
        let own = own_bounds(self.tcx, self.krate.foreign, self.krate.given, id);
        let mut own_params = Vec::new();
        for bound in bounds(self.tcx, self.krate.foreign, self.krate.given, id) {
            if own.contains(&bound) {
                let word = format!(
                    "{}{}",
                    evidence_word(self.tcx, bound.self_ty()),
                    trait_word(self.tcx, bound)
                );
                let name = self.fresh(Base::Named(&js_word(&word)));
                specialized.push((bound, Expr::var(&name)));
                // And as the body asks for it, in the impl's terms: a method's
                // `where Self: PartialEq` of a `Wrapping<T>`'s.
                let concrete = ty::EarlyBinder::bind(self.tcx, bound)
                    .instantiate(self.tcx, args)
                    .skip_normalization();
                if concrete != bound {
                    specialized.push((concrete, Expr::var(&name)));
                }
                own_params.push((bound, name));
                continue;
            }
            let concrete = ty::EarlyBinder::bind(self.tcx, bound)
                .instantiate(self.tcx, args)
                .skip_normalization();
            specialized.push((bound, self.dictionary(concrete, span)?));
        }
        // And the impl's own, `SClone` of `impl<S: Clone> Movable<S> for
        // Point<S>`: a call on the impl's types, which the body's resolve
        // to, is given them.
        specialized.extend(self.given.evidence.iter().cloned());
        // Its body is in the impl's terms (`default_args`), so it drops as
        // the impl does, with the impl's drops (ADR 0098); and its own type
        // parameters with those its caller gives after its dictionaries, as
        // the trait declares them (ADR 0163, #802), each by its type in those
        // terms, which no drop of the impl's is (#804, #887).
        let (mut drops, mut slots) = self.impl_drops_for_default();
        let mut own_drops = Vec::new();
        for index in self.own_drops_taken(id, id) {
            let ty = drop_slot(self.tcx, self.slots(), id, index);
            let name = self.fresh(Base::Prefixed(Role::Drop, &drop_word(self.tcx, ty)));
            let in_impl = ty::EarlyBinder::bind(self.tcx, ty)
                .instantiate(self.tcx, args)
                .skip_normalization();
            if !crate::lower::recognition::known_by_bounds(in_impl) {
                continue;
            }
            drops.insert(in_impl, crate::lower::drops::GivenDrop::lent(name.clone()));
            own_drops.push((index, name, in_impl));
        }
        // And each it could be given, of its own where clause too (ADR 0489).
        slots.extend(
            (drop_items(self.tcx, self.krate.foreign, id).into_iter()).filter_map(|own| match *ty::EarlyBinder::bind(
                self.tcx, own,
            )
            .instantiate(self.tcx, args)
            .skip_normalization()
            .kind()
            {
                ty::Alias(_, alias) => Some(alias),
                _ => None,
            }),
        );
        // Its body in the impl's terms names the impl's const parameters,
        // `CAP` of an `ArrayVec<T, CAP>`, the impl's, and its own, given after
        // its arguments, numbered after the impl's (ADR 0135, `default_args`).
        let mut consts = self.impl_const_params();
        let generics = self.tcx.generics_of(id);
        // A C-variadic default's own, which can't come after what it's given
        // past the rest (ADR 0428).
        if crate::lower::recognition::is_c_variadic(self.tcx, id)
            && (!own_params.is_empty() || !own_const_params(self.tcx, id).is_empty())
        {
            return Err(self.unsupported(span, "a C-variadic default given a dictionary or a const"));
        }
        let mut const_names = Vec::new();
        for index in own_const_params(self.tcx, id) {
            let name = self.fresh(Base::Named(generics.param_at(index as usize, self.tcx).name.as_str()));
            // As the impl's terms number it (`default_args`).
            if let Some(there) = args.get(index as usize).and_then(|arg| arg.as_const())
                && let ty::ConstKind::Param(there) = there.kind()
            {
                consts.push((there.index, Expr::var(&name)));
            }
            const_names.push((index, name));
        }
        // The facts its caller gives of its own type parameters, `KMap`,
        // after its dictionaries (ADR 0466).
        let mut facts: Vec<(Ty<'tcx>, TypeFact, Expr)> = Vec::new();
        let mut fact_names = Vec::new();
        for &(index, fact) in self.krate.type_facts.get(&id).into_iter().flatten() {
            if index < generics.parent_count as u32 {
                continue;
            }
            let of = drop_slot(self.tcx, self.slots(), id, index);
            if self
                .given
                .type_facts
                .iter()
                .chain(&facts)
                .any(|&(at, given, _)| at == of && given == fact)
            {
                return Err(self.unsupported(span, "a trait default's type parameter's fact here"));
            }
            let name = self.fresh(Base::Named(&format!("{}{}", drop_word(self.tcx, of), fact.word())));
            facts.push((of, fact, Expr::var(&name)));
            // And in the impl's terms, as the body asks: `T::Item` of the
            // trait's `Self::Item`.
            let concrete = ty::EarlyBinder::bind(self.tcx, of)
                .instantiate(self.tcx, args)
                .skip_normalization();
            if concrete != of {
                facts.push((concrete, fact, Expr::var(&name)));
            }
            fact_names.push(((index, fact), name));
        }
        // A method's, or a constant's initializer, a trait's default (ADR 0176).
        let body = match self.krate.bodies.get(&id) {
            Some(body) => *body,
            // A library's trait of classes' default, its consumer's class's
            // prototype's method, of the MIR its library keeps, which the
            // crate compiles where it's asked (ADR 0914, #1073).
            None if self.asked_of_library(id) => {
                self.krate.asked.borrow_mut().push((id, self.module));
                return Ok(Expr::undefined());
            }
            None => (self.krate.closures.get(&id).copied())
                .ok_or_else(|| self.unsupported(span, "a trait's default of another crate"))?,
        };
        // Where it's called, after its arguments, of one that takes it, which
        // `Location::caller()` in it reads (ADR 0427).
        let location = self.takes_caller_location(id).then(|| self.fresh(Role::Location));
        let nested = crate::lower::Nested::Default {
            given: Box::new(super::DefaultGiven {
                evidence: specialized,
                self_args: args,
                const_params: consts,
                caller_location: location.as_ref().map(|name| (id, Expr::var(name))),
                type_facts: facts,
            }),
            typing_env: default_typing_env(self.tcx, imp, id, args),
            drops,
            slots,
        };
        let mut rest = Vec::new();
        // From its MIR (ADR 0364), a constant's initializer of nothing, its
        // value given back. From its MIR, as the trait has it, `Self` the
        // impl's type where it's asked (ADR 0364). Only the drops it uses:
        // most defaults drop nothing of their `Self`.
        // A trait of classes' default, its class's prototype's method, as
        // the class has it: `Self` the class, and each method its impl
        // gives a field by that field, read in place (rd283).
        let class_default = (body.mir.as_ref()).map(|mir| {
            let mut mir = default_mir(self.tcx, mir, args, default_typing_env(self.tcx, imp, id, args));
            if crate::lower::classes::is_class_trait(self.tcx, self.tcx.parent(id)) {
                mir.read_fields(self.tcx, ty::TypingEnv::fully_monomorphized());
            }
            mir
        });
        // What its caller through the dictionary gives, in its trait's forms,
        // made what its body, in the impl's terms, takes (`entry_forms`):
        // `self.value` of a `&mut Self` generic code has in a box (#803).
        // Of a method: a constant's initializer takes nothing.
        let forms = match self.tcx.def_kind(id) {
            DefKind::AssocFn => {
                let count = (self.tcx.fn_sig(id).instantiate_identity())
                    .skip_normalization()
                    .skip_binder()
                    .inputs()
                    .len();
                Some(self.entry_forms(id, (id, args), args, count, Some(imp)))
            }
            _ => None,
        };
        if (forms.as_ref())
            .is_some_and(|forms| forms.result_cell || forms.result_within.as_ref().is_some_and(Within::changes))
        {
            return Err(self.unsupported(span, "a copied default giving back a `&mut` its impl holds otherwise"));
        }
        // Each parameter it makes another's: named in its body's names, before
        // its body is, so no name of its body's is one of them, a box of a
        // `self` as the naming owner names one (`self_box_name`).
        let count = forms.as_ref().map_or(0, |forms| forms.unboxed.len());
        let words: Vec<Option<&str>> = (0..count)
            .map(|i| {
                let forms = forms.as_ref()?;
                let made = forms.unboxed[i] > 0
                    || forms.rebound[i].is_some()
                    || (forms.within[i].as_ref())
                        .is_some_and(|within| within.toward_impl(&Expr::undefined()).is_some());
                let receiver = || {
                    let sig = self.tcx.fn_sig(id).instantiate(self.tcx, args).skip_normalization();
                    sig.skip_binder().inputs()[0]
                };
                made.then(|| match i {
                    0 => crate::lower::analysis::self_box_name(self.tcx, receiver()),
                    _ => "arg",
                })
            })
            .collect();
        let ((signature, used, given_names), hoisted) = self.lowered_in(body, id, nested, |cx| {
            let given_names: Vec<Option<String>> = (words.iter())
                .map(|word| word.map(|word| cx.fresh(Base::Named(word))))
                .collect();
            let signature = match class_default.as_ref().or(body.mir.as_ref()) {
                Some(mir) => cx.mir_signature(body, mir, &mut rest)?,
                None => return Err(cx.unsupported(span, "this default, without its MIR")),
            };
            Ok((signature, cx.used_drops(), given_names))
        })?;
        // After its arguments, what its caller gives of its own, in
        // `evidence_slots`' order, as `own_evidence` gives them: of its
        // trait's parameters no drops, which its impl's are, made here.
        let (mut params, is_async) = signature;
        let mut prologue = Vec::new();
        for (i, param) in params.iter_mut().enumerate().take(count) {
            let (Some(forms), Some(given)) = (forms.as_ref(), given_names[i].clone()) else {
                continue;
            };
            let js::Pattern::Name(name) = param else {
                continue;
            };
            let mut value = Expr::var(&given);
            for _ in 0..forms.unboxed[i] {
                value = Expr::held(value);
            }
            if let Some(crossing) = forms.rebound[i] {
                value = self.form_back(crate::lower::generic_facts::Form::HeldBox, crossing, value);
            }
            if let Some(made) = (forms.within[i].as_ref()).and_then(|within| within.toward_impl(&value)) {
                value = made;
            }
            if value != Expr::var(&given) {
                // A `const` but of a parameter its body assigns again, `mut`.
                let mutable = (class_default.as_ref()).is_some_and(|mir| {
                    mir.body.local_decls[rustc_middle::mir::Local::from_usize(i + 1)].mutability
                        == rustc_hir::Mutability::Mut
                });
                prologue.push(
                    match mutable {
                        true => StmtKind::Let(name.clone(), Some(value)),
                        false => StmtKind::Const(name.clone(), value),
                    }
                    .at(js::Span::NONE),
                );
                *param = given.into();
            }
        }
        let named = |names: &[(u32, String)], at: u32| names.iter().find(|&&(i, _)| i == at).map(|(_, n)| n.clone());
        // Its own drops last, those after the last it uses left out, as its
        // caller's `undefined`s at the end are.
        let own_used = |key: Ty<'tcx>| used.contains(&key);
        let mut unused = 0;
        for slot in self.evidence_slots(id, Scope::Own) {
            let name = match slot {
                Slot::Location => location.clone(),
                Slot::Const(index) => named(&const_names, index),
                Slot::Bound(bound) => own_params.iter().find(|&&(b, _)| b == bound).map(|(_, n)| n.clone()),
                // By its fact too: a parameter's `Size` and `Align` are two.
                Slot::Fact(index, fact) => (fact_names.iter())
                    .find(|&&(at, _)| at == (index, fact))
                    .map(|(_, name)| name.clone()),
                Slot::Drop(index) => {
                    // None, of a crate whose values drop nothing.
                    let Some((_, name, key)) = own_drops.iter().find(|(at, _, _)| *at == index) else {
                        continue;
                    };
                    unused = match own_used(*key) {
                        true => 0,
                        false => unused + 1,
                    };
                    Some(name.clone())
                }
            };
            params.push(name.expect("each of a default's own slots named").into());
        }
        params.truncate(params.len() - unused);
        rest.splice(0..0, hoisted);
        let mut out = prologue;
        out.extend(rest);
        Ok(if is_async {
            Expr::async_arrow(params, out)
        } else {
            Expr::arrow(params, out)
        })
    }
}
