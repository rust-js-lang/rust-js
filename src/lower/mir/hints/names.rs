//! What tidy is told of a body's names: each temporary MIR made, by the
//! names a Rust developer's source gives what it holds, and each local by
//! its type's. Split from hints.rs by concept, as a file is at most 1500
//! lines.

use super::*;

impl<'tcx> FnCx<'_, 'tcx> {
    /// Each temporary MIR made, `_5`, by the names a Rust developer's
    /// tooling suggests for it, rust-analyzer's, first to last (ADR 0569):
    /// the method or function that made it, `trailingZeros` of
    /// `a.trailing_zeros()`, `uppercase` of `s.to_uppercase()`, but for one
    /// that names nothing, `unwrap`, `clone`, `new`, `from_str`, and one the
    /// source doesn't call by its name, a macro's, an operator's or a
    /// closure's; then the program's type it holds, `point` of an
    /// `Option<Point>`. One that takes or copies another temporary, `&_5`
    /// as `format_args!` takes what it shows, has that one's names.
    pub(in crate::lower::mir) fn temporaries_names(
        &self,
        mir_body: &mir::Body<'tcx>,
        state: &State<'_, 'tcx>,
    ) -> IndexMap<String, Vec<String>> {
        let called = self.calls_named_in_source(mir_body);
        let mut taken_from: IndexMap<Local, Local> = IndexMap::default();
        for data in mir_body.basic_blocks.iter() {
            for statement in &data.statements {
                if let StatementKind::Assign(assign) = &statement.kind
                    && let Some(local) = assign.0.as_local()
                    && let Rvalue::Ref(_, _, from) | Rvalue::Use(Operand::Copy(from) | Operand::Move(from), ..) =
                        &assign.1
                    && let Some(from) = from.as_local()
                {
                    taken_from.insert(local, from);
                }
            }
        }
        (state.locals.names.iter_enumerated())
            .filter(|(_, name)| super::tidy::is_temporary(name))
            .map(|(local, name)| {
                // What it was taken from, as far as that goes.
                let mut at = local;
                for _ in 0..mir_body.local_decls.len() {
                    match taken_from.get(&at) {
                        Some(&from) if from != at && !called.contains_key(&at) => at = from,
                        _ => break,
                    }
                }
                let suggested = [
                    called.get(&at).and_then(|&def_id| self.call_name(def_id)),
                    self.held_type_name(mir_body.local_decls[local].ty),
                ];
                let names = (suggested.into_iter().flatten())
                    .map(|name| variable_name(&crate::lower::camel_case(&name)))
                    .filter(|name| crate::names::js_ident(name) == *name)
                    .collect();
                (name.clone(), names)
            })
            .collect()
    }

    /// What rust-analyzer names a call's result: a method's name without a
    /// conversion's prefix, `uppercase` of `to_uppercase`, and none of a
    /// method that names nothing; a function's without a constructor's
    /// prefix, `config` of `from_config`; and none of a name that's no
    /// noun of what it gives, `new`, `default`, `str`.
    pub(super) fn call_name(&self, def_id: rustc_span::def_id::DefId) -> Option<String> {
        const USELESS_METHODS: &[&str] = &[
            "to_string",
            "as_str",
            "to_owned",
            "as_ref",
            "clone",
            "cloned",
            "expect",
            "expect_none",
            "unwrap",
            "unwrap_none",
            "unwrap_or",
            "unwrap_or_default",
            "unwrap_or_else",
            "unwrap_unchecked",
            "iter",
            "into_iter",
            "iter_mut",
            "into_future",
            // And the `Err` side of what rust-analyzer's list has, as it names
            // nothing either.
            "unwrap_err",
            "expect_err",
        ];
        const USELESS_NAMES: &[&str] = &[
            "new", "default", "option", "some", "none", "ok", "err", "str", "string", "from", "into",
        ];
        let name = self.tcx.item_name(def_id);
        let name = name.as_str();
        let method = self
            .tcx
            .opt_associated_item(def_id)
            .is_some_and(|item| item.is_method());
        let name = match method {
            true if USELESS_METHODS.contains(&name) => return None,
            true => ["into_", "as_", "to_"]
                .iter()
                .find_map(|p| name.strip_prefix(p))
                .unwrap_or(name),
            false => ["from_", "with_", "into_"]
                .iter()
                .find_map(|p| name.strip_prefix(p))
                .unwrap_or(name),
        };
        let plain = name.starts_with(|c: char| c.is_ascii_lowercase()) && !name.starts_with('_');
        let keyword = rustc_span::symbol::Ident::from_str(name).is_reserved();
        (plain && !keyword && !USELESS_NAMES.contains(&name)).then(|| name.to_string())
    }

    /// The name a value of `ty` is kept by, its type's, `order` of an
    /// `Order`, through a reference and what JS holds as the value it
    /// holds, an `Option`, a `Box`, an `Rc`, a `Cell` and a `RefCell` of the
    /// value itself: of the program's own or a library's type, `None` of
    /// std's. Not a `Result`'s `Ok`, as rust-analyzer's is: JS holds a
    /// `Result` as `{ TAG, _0 }`, which `order.TAG` would misname.
    fn held_type_name(&self, ty: Ty<'tcx>) -> Option<String> {
        let mut ty = ty.peel_refs();
        loop {
            ty = match ty.kind() {
                _ if let Some(inner) = self.option_of(ty) => inner.peel_refs(),
                ty::Adt(_, args) if ty.is_box() || self.is_std_type(ty, StdItem::Rc) => args.type_at(0).peel_refs(),
                // A `Cell` a temporary holds, and a `RefCell` of the value
                // itself, are what they hold: `Cell::new(Some(error))`'s
                // `error` (rd413).
                ty::Adt(_, args) if self.is_std_type(ty, StdItem::Cell) || self.is_value_cell(ty) => {
                    args.type_at(0).peel_refs()
                }
                _ => break,
            };
        }
        let ty::Adt(adt, _) = ty.kind() else { return None };
        if crate::lower::recognition::is_std_crate(self.tcx, adt.did()) {
            return None;
        }
        Some(self.tcx.item_name(adt.did()).to_string())
    }

    /// The names each of whose locals is a `bool`, a shared reference to one
    /// too, which JS reads as it, of a closure what it captures too; the
    /// names each of whose locals is an integer; and those each of whose
    /// locals is a float, a JS number too (ADR 0966).
    pub(in crate::lower::mir) fn typed_names(
        &self,
        mir_body: &mir::Body<'tcx>,
        state: &State<'_, 'tcx>,
    ) -> (IndexSet<String>, IndexSet<String>, IndexSet<String>) {
        // A name each of whose locals is a `bool`, and one each of whose
        // locals is an integer.
        let mut booleans = IndexSet::default();
        let mut others = IndexSet::default();
        let mut numbers = IndexSet::default();
        let mut not_numbers = IndexSet::default();
        let mut floats = IndexSet::default();
        let mut not_floats = IndexSet::default();
        for (local, name) in state.locals.names.iter_enumerated().filter(|(_, n)| !n.is_empty()) {
            // A shared reference to one too, `*show_hint`'s, which JS reads as it.
            let ty = mir_body.local_decls[local].ty;
            match ty.is_integral() {
                true => numbers.insert(name.clone()),
                false => not_numbers.insert(name.clone()),
            };
            match ty.is_floating_point() {
                true => floats.insert(name.clone()),
                false => not_floats.insert(name.clone()),
            };
            let bool = match ty.kind() {
                ty::Ref(_, inner, ty::Mutability::Not) => inner.is_bool(),
                _ => ty.is_bool(),
            };
            match bool {
                true => booleans.insert(name.clone()),
                false => others.insert(name.clone()),
            };
        }
        // And a closure's, of what it captures: its parent's.
        let env = mir_body
            .args_iter()
            .next()
            .map(|local| mir_body.local_decls[local].ty.peel_refs());
        if let Some(ty::Closure(_, args)) = env.map(|ty| ty.kind()) {
            for (ty, (capture, _)) in args.as_closure().upvar_tys().iter().zip(state.facts.captures()) {
                if let js::ExprKind::Var(name) = &capture.kind {
                    match ty.peel_refs().is_bool() {
                        true => booleans.insert(name.clone()),
                        false => others.insert(name.clone()),
                    };
                }
            }
        }
        booleans.retain(|name| !others.contains(name));
        numbers.retain(|name| !not_numbers.contains(name));
        floats.retain(|name| !not_floats.contains(name));
        (booleans, numbers, floats)
    }
}

/// A type's name as a variable's, lower camel case, as JS names one:
/// `order` of `Order`, `httpServer` of `HTTPServer`, `url` of `URL`.
pub(super) fn variable_name(type_name: &str) -> String {
    let chars: Vec<char> = type_name.chars().collect();
    // The capitals it starts with, but for one that starts the next word.
    let run = chars.iter().take_while(|c| c.is_ascii_uppercase()).count();
    let lowered = match run > 1 && run < chars.len() {
        true => run - 1,
        false => run,
    };
    (chars.iter().enumerate())
        .map(|(i, c)| match i < lowered.max(1) {
            true => c.to_ascii_lowercase(),
            false => *c,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::variable_name;

    // A type's name as a variable's: its first capital lowered, a run of them
    // too, but for the one that starts the next word.
    #[test]
    fn a_type_name_is_a_variable_name() {
        assert_eq!(variable_name("Order"), "order");
        assert_eq!(variable_name("HTTPServer"), "httpServer");
        assert_eq!(variable_name("URL"), "url");
        assert_eq!(variable_name("IOError"), "ioError");
        assert_eq!(variable_name("trailingZeros"), "trailingZeros");
    }
}
