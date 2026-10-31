// Mutations of src/lower/traits/dyns.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "dyn-tail-holder-unchecked",
    breaks:
      "a struct whose last field is a `dyn` behind a `Mutex` compiles, never given its `impl`, so its call reads `undefined.grow`, not refused (#375)",
    file: "src/lower/traits/dyns.rs",
    find: "                || !self.is_own_struct(self.tail_holder(target)))\n",
    replace: "                || false)\n",
    tests: ["test/limitations.test.ts", "-t", "behind a .Mutex"],
  },
  {
    name: "dyn-tail-in-a-cell-unpeeled",
    breaks:
      "a struct whose last field is made a `dyn` in a `RefCell` that's the struct, `Rc<RefCell<Framed<dyn Shape>>>`, isn't given its `impl`, so a borrow's call reads `undefined.grow` (#375)",
    file: "src/lower/traits/dyns.rs",
    find: "            ty::Adt(_, args) if self.is_value_cell(pointee) => args.type_at(0),\n",
    replace:
      "            ty::Adt(_, args) if false && self.is_value_cell(pointee) => args.type_at(0),\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_tail_in_a_ref_cell.rs$"],
  },
  {
    name: "dyn-bound-lifetimes",
    breaks:
      "a `dyn for<'a>` trait's dictionary is asked of rustc with its lifetime bound, and rustc panics",
    file: "src/lower/traits/dyns.rs",
    find: "                self.tcx\n                    .instantiate_bound_regions_with_erased(p.with_self_ty(self.tcx, self_ty))",
    replace: "                p.with_self_ty(self.tcx, self_ty).skip_binder()",
    tests: ["test/corpus.test.ts", "-t", "higher_ranked_dyn"],
  },
  {
    name: "own-pointer-unsize",
    breaks:
      "a pointer of the crate's own, unsized to a `dyn`, is the value it was, not a `dyn`'s",
    file: "src/lower/traits/dyns.rs",
    find: "        if let ty::Adt(adt, _) = target.kind()\n            && (adt.did().is_local() || self.krate.foreign.in_library(adt.did()))\n",
    replace:
      "        if let ty::Adt(adt, _) = target.kind()\n            && (adt.did().is_local() || self.krate.foreign.in_library(adt.did()))\n            && false\n",
    tests: ["test/diagnostics.test.ts", "-t", "pointer of the crate"],
  },
  {
    name: "dyn-of-cell-copied",
    breaks:
      "`&mut n as &mut dyn Counter` holds the handle as its `value`, and `bump` writes the handle's field, not `n`",
    file: "src/lower/traits/dyns.rs",
    find: "        if self.is_cell(source) {\n",
    replace: "        if false && self.is_cell(source) {\n",
    tests: ["test/corpus.test.ts", "-t", "dyn_mut"],
  },
  {
    name: "dyn-upcast-copied",
    breaks:
      "`up(s).bump()` of a `&mut dyn Sub` writes a copy of its `value`, and `n` never changes",
    file: "src/lower/traits/dyns.rs",
    find: "            if matches!(target.kind(), ty::Ref(_, _, Mutability::Mut)) {\n",
    replace:
      "            if false && matches!(target.kind(), ty::Ref(_, _, Mutability::Mut)) {\n",
    tests: ["test/corpus.test.ts", "-t", "dyn_mut"],
  },
  {
    name: "parse-error-dyn-refused",
    breaks:
      "`s.parse::<i64>()?` into a `Box<dyn Error>` is a pair of the dictionary ADR 0186 gives std's parse errors, where it's its message, `$parseErrorDyn(\"ParseIntError\")`",
    snapshots: true,
    file: "src/lower/traits/dyns.rs",
    find: "        if self.is_parse_error(ty)\n            && let ty::Adt(adt, _) = ty.kind()",
    replace: "        if false\n            && let ty::Adt(adt, _) = ty.kind()",
    tests: ["test/corpus.test.ts", "-t", "std_errors_boxed"],
  },
  {
    name: "json-error-dyn-refused",
    breaks: "`serde_json::from_str(s)?` into a `Box<dyn Error>` is an error",
    file: "src/lower/traits/dyns.rs",
    find: "        if self.is_json_error(ty) {\n            return Some(",
    replace: "        if false {\n            return Some(",
    tests: ["test/serde.test.ts", "-t", "Box<dyn Error> shows as serde_json"],
  },
  {
    name: "dyn-cell-refused",
    breaks:
      "a `RefCell<dyn T>` is refused, as a struct whose last field is a `dyn`",
    file: "src/lower/traits/dyns.rs",
    find: "            ty::Dynamic(predicates, ..) => predicates.principal_def_id().filter(|&id| self.is_rust_trait(id)),\n            _ => None,\n",
    replace: "            ty::Dynamic(..) => None,\n            _ => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_ref_cells.rs$"],
  },
  {
    name: "dyn-cell-of-fn-mut",
    breaks:
      "a `RefCell<dyn FnMut>` is taken for a pair, and its guard's `value` called, where it's the function",
    file: "src/lower/traits/dyns.rs",
    find: "            ty::Dynamic(predicates, ..) => predicates.principal_def_id().filter(|&id| self.is_rust_trait(id)),\n            _ => None,\n",
    replace:
      "            ty::Dynamic(predicates, ..) => predicates.principal_def_id(),\n            _ => None,\n",
    tests: ["test/corpus.test.ts", "-t", "^unsized_struct_fn.rs$"],
  },
  {
    name: "dyn-cell-literal-helper",
    breaks:
      "a `RefCell<dyn T>` made where it's coerced is `$dynCell({ value }, impl)`, not `{ value, impl }`",
    file: "src/lower/traits/dyns.rs",
    find: "        if let js::ExprKind::Object(props, ..) = &holder.kind\n",
    replace:
      "        if let js::ExprKind::Object(props, ..) = &Expr::undefined().kind\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_ref_cells.rs$"],
    snapshots: true,
  },
  {
    name: "counted-dyn-cell-unrefused",
    breaks:
      "a `Weak<RefCell<dyn T>>` is made as an `Rc`'s, not refused, and its upgrade fails",
    file: "src/lower/traits/dyns.rs",
    find: "            if self.counted_weak(target).is_some() {\n                return Err(",
    replace: "            if false {\n                return Err(",
    tests: ["test/diagnostics.test.ts", "-t", "a Weak of a RefCell of a dyn"],
  },
  {
    name: "dyn-cell-clash-unchecked",
    breaks:
      "a cell made a `dyn` of two traits with a method of one name is given one `impl` that calls the other's",
    file: "src/lower/traits/dyns.rs",
    find: "                && !methods(other).is_disjoint(&named)\n",
    replace: "                && false\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a RefCell made a dyn of two traits of one method name",
    ],
  },
  {
    name: "dyn-cell-clash-of-supertrait",
    breaks:
      "a cell made a `dyn` of a trait and of its supertrait, whose methods are the same, is refused",
    file: "src/lower/traits/dyns.rs",
    find: "                && !under(trait_id, other)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_ref_cell_supertrait.rs$"],
  },
  {
    name: "dyn-cell-clash-of-subtrait",
    breaks:
      "a cell made a `dyn` of a supertrait and of a trait under it, whose methods are the same, is refused",
    file: "src/lower/traits/dyns.rs",
    find: "                && !under(other, trait_id)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_ref_cell_supertrait.rs$"],
  },
  {
    name: "dyn-cell-clash-of-bound",
    breaks:
      "a generic cell made a `dyn` of two of its bounds with a method of one name is given one `impl` that calls the other's",
    file: "src/lower/traits/dyns.rs",
    find: "                && (bounds.contains(&other) || self.has_user_impl(other, held))\n",
    replace: "                && self.has_user_impl(other, held)\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a generic RefCell made a dyn of two traits of one method name",
    ],
  },
  {
    name: "dyn-cell-upcast-unrefused",
    breaks:
      "a `RefCell<dyn Sub>` as a `RefCell<dyn Base>` is given a dictionary that takes a pair, and its call fails",
    file: "src/lower/traits/dyns.rs",
    find: "            if matches!(from.type_at(0).kind(), ty::Dynamic(..)) {\n",
    replace: "            if false {\n",
    tests: ["test/diagnostics.test.ts", "-t", "a RefCell of a dyn upcast"],
  },
  {
    name: "dyn-tail-unrefused",
    breaks:
      "an `Rc<Fat<dyn T>>`, which may be counted, whose last field would be dropped through its `impl`, isn't refused",
    file: "src/lower/traits/dyns.rs",
    find: "            && (!self.holds_a_dyn_tail(target)\n                || self.is_dyn_debug(tail)\n",
    replace:
      "            && (false\n                || self.is_dyn_debug(tail)\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a raw pointer to a struct of a dyn",
    ],
  },
  {
    name: "dyn-tail-parent-root",
    breaks:
      "a `&Fat<Fat<dyn T>>`'s `impl` is given the outer struct, not the one the `dyn` is in",
    file: "src/lower/traits/dyns.rs",
    find: "            path.push((ty, adt.non_enum_variant().fields.len() - 1));\n",
    replace: "            return path;\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_struct_tails.rs$"],
  },
  {
    name: "dyn-tail-reborrow-refused",
    breaks:
      "a `&Fat<dyn T>` reborrowed as one, `foo(&x.ptr)`, is refused as an upcast",
    file: "src/lower/traits/dyns.rs",
    find: "                if erased(concrete) == erased(tail) {\n",
    replace: "                if erased(concrete) == erased(tail) && false {\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_struct_tails.rs$"],
  },
  {
    name: "counted-rc-unsized-around",
    breaks:
      "a counted `Rc` made an `Rc<dyn T>` is a pair around the `Rc`, which each use reads as an `Rc` of a pair",
    file: "src/lower/traits/dyns.rs",
    find: "        if self.counted_rc(source).is_some() || self.counted_weak(source).is_some() {\n            return self.counted_unsized(source, target, value, span, out);\n        }\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^rc_dyn_destructors.rs$"],
  },
  {
    name: "counted-rc-same-dyn-refused",
    breaks:
      "an `Rc<dyn T>` or a `Weak<dyn T>` unsized to itself, a slice's, is refused",
    file: "src/lower/traits/dyns.rs",
    find: "        if self.tcx.erase_and_anonymize_regions(source) == self.tcx.erase_and_anonymize_regions(target) {\n            return Ok(value);\n        }\n        if let Some(pointee) = self.counted_rc(source)",
    replace: "        if let Some(pointee) = self.counted_rc(source)",
    tests: ["test/corpus.test.ts", "-t", "^rc_dyn_upcast_counted.rs$"],
    snapshots: true,
  },
  {
    name: "auto-trait-upcast-repaired",
    breaks:
      "a `dyn T + Send` as a `dyn Send` is a pair around its pair, whose dictionary has no `$drop`, so the value is never dropped",
    file: "src/lower/traits/dyns.rs",
    find: "            let Some(to) = self.dyn_trait_ref(target, self_ty) else {\n                return Ok(value);\n",
    replace:
      "            let Some(to) = self.dyn_trait_ref(target, self_ty) else {\n                return Ok(Expr::object(vec![Prop::Field(js::HELD.into(), value), Prop::Field(js::DICTIONARY.into(), Expr::object(Vec::new()))]));\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_auto_traits.rs$"],
  },
  {
    name: "auto-trait-dictionary-dropless",
    breaks:
      "a `dyn Send`'s dictionary is `{}`, without its value's drop, which then never runs",
    file: "src/lower/traits/dyns.rs",
    find: "            None => Ok(Expr::object(self.dictionary_drop(self_ty, span)?.into_iter().collect())),\n",
    replace: "            None => Ok(Expr::object(Vec::new())),\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_auto_traits.rs$"],
  },
  {
    name: "auto-trait-tail-owned",
    breaks:
      "a `*const Fat<dyn Send>`, whose last field is a `dyn` of no trait, is made where no pointer to one can hold it",
    file: "src/lower/traits/dyns.rs",
    find: "                if self.recognition().is_pair_dyn(traits) || self.is_dyn_debug(tail))\n",
    replace:
      "                if traits.principal_def_id().is_some_and(|id| self.is_rust_trait(id)) || self.is_dyn_debug(tail))\n",
    tests: [
      "test/diagnostics.test.ts",
      "-t",
      "a raw pointer to a struct of a dyn last field",
    ],
  },
  {
    name: "boxed-dyn-tail-not-given",
    breaks:
      "a `Box<Fat<dyn Show>>` is the struct with no `impl`, which dropping its last field reads",
    file: "src/lower/traits/dyns.rs",
    find: "        target.is_ref() || target.is_box() || self.is_rc(target)\n",
    replace: "        target.is_ref() || self.is_rc(target)\n",
    tests: ["test/corpus.test.ts", "-t", "^unsized_struct_dyn.rs$"],
  },
  {
    name: "auto-trait-dyn-tail-not-given",
    breaks:
      "a struct whose last field is a `dyn Send` or a `dyn Display` is given no `impl`, which dropping or showing that field reads",
    file: "src/lower/traits/dyns.rs",
    find: "        matches!(tail.kind(), ty::Dynamic(traits, ..) if self.recognition().is_pair_dyn(traits))\n",
    replace:
      "        matches!(tail.kind(), ty::Dynamic(traits, ..) if traits.principal_def_id().is_some_and(|id| self.is_rust_trait(id)))\n",
    tests: ["test/corpus.test.ts", "-t", "^unsized_struct_dyn.rs$"],
  },
  {
    name: "boxed-dyn-tail-refused",
    breaks:
      "a `Box<Fat<dyn Show>>`, a struct whose last field is a `dyn`, is refused, as an `Rc` of one is",
    file: "src/lower/traits/dyns.rs",
    find: "            && (!self.holds_a_dyn_tail(target)\n                || self.is_dyn_debug(tail)\n",
    replace:
      "            && (!target.is_ref()\n                || self.is_dyn_debug(tail)\n",
    tests: ["test/corpus.test.ts", "-t", "^unsized_struct_dyn.rs$"],
  },
  {
    name: "dyn-tail-given-spilled",
    breaks:
      "a struct that holds its `dyn` field itself is spilled before it's given its `impl`, `const value = { .. }; $dynCell(value, ..)`",
    file: "src/lower/traits/dyns.rs",
    find: "            if path.is_empty() && !counted {\n",
    replace: "            if false && path.is_empty() && !counted {\n",
    tests: ["test/corpus.test.ts", "-t", "^unsized_struct_dyn.rs$"],
    snapshots: true,
  },
  {
    name: "unit-dyn-given-undefined",
    breaks:
      "a `()` as a `dyn` JS is given the value of is `undefined`, a trailing one left out: `useEffect(effect)`, run on every render, of `use_effect(effect, Some(&()))`, `[]`",
    file: "src/lower/traits/dyns.rs",
    find: "        let value = match self.pointee(source).is_unit() {\n",
    replace:
      "        let value = match self.pointee(source).is_unit() && false {\n",
    tests: ["test/jsx-types.test.ts", "-t", "JSX hooks take"],
  },
  {
    name: "unit-dyn-given-array",
    breaks:
      "a `()` as a `dyn` of any value is `[]`, not `undefined`: `kinds([], ..)` of `kinds(Some(&()), ..)` (ADR 0775)",
    file: "src/lower/traits/dyns.rs",
    find: "            .unwrap_or(value),\n",
    replace: "            .unwrap_or(Expr::array(Vec::new())),\n",
    tests: ["test/unit-given.test.ts", "-t", "any value is undefined"],
  },
  {
    name: "counted-dyn-cell-refused",
    breaks:
      "a counted `Rc<RefCell<C>>` made an `Rc<RefCell<dyn T>>` is refused, though its cell is shared, given its `impl` in place",
    file: "src/lower/traits/dyns.rs",
    find: "            if self.counted_rc(target).is_some() {\n                let rc = match value.reads_same() {\n",
    replace:
      '            if self.counted_rc(target).is_some() {\n                return Err(self.unsupported(span, "mutant"));\n                #[allow(unreachable_code)]\n                let rc = match value.reads_same() {\n',
    tests: ["test/corpus.test.ts", "-t", "rc_refcell_dyn_shared"],
  },
  {
    name: "dyn-cell-given-generic",
    breaks:
      "a `RefCell<dyn T>`, its pair, is given to generic code that reads it as a cell of a value, and JS throws",
    file: "src/lower/traits/dyns.rs",
    find: "                && self.cell_dyn_trait(given).is_some()\n",
    replace: "                && false\n",
    tests: ["test/limitations.test.ts", "-t", "RefCell. of a .dyn"],
  },
  {
    name: "dyn-tail-through-wrapper",
    breaks:
      "a struct's last field a `ManuallyDrop<dyn T>` is taken for a struct of its own, given the `impl`",
    file: "src/lower/traits/dyns.rs",
    find: "            let field = self.unwrapped(self.field_ty(last, args));\n",
    replace: "            let field = self.field_ty(last, args);\n",
    tests: ["test/corpus.test.ts", "-t", "manually_drop_unsized"],
  },
  {
    name: "dyn-shown-of-read-pair-refused",
    breaks:
      "a struct's `dyn` last field, shown, is refused as made where it's shown",
    file: "src/lower/traits/dyns.rs",
    find: "        if !pair.parts_read_same() {\n",
    replace: "        if !pair.reads_same() {\n",
    tests: ["test/corpus.test.ts", "-t", "manually_drop_unsized"],
  },
  {
    name: "rc-dyn-tail-refused",
    breaks: "an `Rc` of a struct whose last field is a `dyn` is refused",
    file: "src/lower/traits/dyns.rs",
    find: "        target.is_ref() || target.is_box() || self.is_rc(target)\n",
    replace: "        target.is_ref() || target.is_box()\n",
    tests: ["test/corpus.test.ts", "-t", "rc_dyn_struct_tail"],
  },
  {
    name: "counted-rc-dyn-tail-uncelled",
    breaks:
      "a counted `Rc` of a struct whose last field is a `dyn` is given the `impl` itself, not its struct",
    file: "src/lower/traits/dyns.rs",
    find: "                true => Expr::member(value.clone(), js::HELD),\n",
    replace: "                true => value.clone(),\n",
    tests: ["test/corpus.test.ts", "-t", "rc_dyn_struct_tail"],
  },
  {
    name: "fn-dyn-unwrapped",
    breaks:
      "a type parameter's function made a `dyn Fn` doesn't carry its drop",
    file: "src/lower/traits/dyns.rs",
    find: "            if carried || handed || self.drops(function) == crate::lower::drops::Drops::Nothing {\n",
    replace:
      "            if true || carried || handed || self.drops(function) == crate::lower::drops::Drops::Nothing {\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_generic_drops.rs$"],
  },
  {
    name: "pointee-not-of-raw-pointer",
    breaks:
      "a raw pointer's pointee is the pointer, so `*const dyn Trait` isn't unsized to a pair",
    file: "src/lower/traits/dyns.rs",
    find: "            _ if let Some((inner, _)) = crate::lower::pointers::raw_pointee(self.tcx, ty) => inner,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_pointers.rs$"],
  },
  {
    name: "dyn-of-generic-trait-refused",
    breaks:
      "`&dyn Convert<String>` of a generic trait is refused, as if its parameters had no dictionary",
    file: "src/lower/traits/dyns.rs",
    find: "    pub(in crate::lower) fn dyn_supported(&self, id: DefId) -> bool {\n        self.tcx\n            .associated_items(id)\n",
    replace:
      "    pub(in crate::lower) fn dyn_supported(&self, id: DefId) -> bool {\n        self.tcx.generics_of(id).own_params.len() <= 1\n            && self.tcx\n            .associated_items(id)\n",
    tests: ["test/corpus.test.ts", "-t", "generic_traits"],
  },
  {
    name: "counted-rc-upcast-refused",
    breaks: "a counted `Rc<dyn Sub>` as an `Rc<dyn Super>` is refused",
    file: "src/lower/traits/dyns.rs",
    find: "            && self.is_pair_dyn(pointee)\n            && let (Some(from), Some(to))",
    replace:
      "            && self.is_pair_dyn(pointee)\n            && false\n            && let (Some(from), Some(to))",
    tests: ["test/corpus.test.ts", "-t", "^rc_dyn_upcast_counted.rs$"],
  },
  {
    name: "counted-rc-own-trait-viewed",
    breaks: "an `Rc<dyn Sub + Send>` as an `Rc<dyn Sub>` is a view of itself",
    file: "src/lower/traits/dyns.rs",
    find: "            if from == to {\n                return Ok(value);\n            }\n",
    replace: "",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^rc_dyn_auto_trait_dropped.rs$"],
  },
  {
    name: "handed-closure-given-its-drop",
    breaks:
      "a closure made a `dyn Fn` to hand to JS is given its drop where it's made one, `$dropping(f, drop)`, which nothing runs (ADR 0873)",
    file: "src/lower/traits/dyns.rs",
    find: "            if carried || handed || self.drops(function)",
    replace: "            if carried || self.drops(function)",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_handed_to_js.rs$"],
    snapshots: true,
  },
  {
    name: "fn-made-dyn-fn-once-unconsumed",
    breaks:
      "a `Fn` closure made a `Box<dyn FnOnce()>` and called never drops what it took, where std's `call_once` consumes it (#409)",
    file: "src/lower/traits/dyns.rs",
    find: "            if once && !handed && dropping {",
    replace: "            if false && once && !handed && dropping {",
    tests: ["test/corpus.test.ts", "-t", "^fn_boxed_once_drops.rs$"],
  },
  {
    name: "fn-once-closure-consumed-twice",
    breaks:
      "an `FnOnce` closure made a `dyn FnOnce` drops what it took in its body and again as its call returns (#409)",
    file: "src/lower/traits/dyns.rs",
    find: "                && !matches!(function.kind(), ty::Closure(_, args) if args.as_closure().kind() == ty::ClosureKind::FnOnce)\n",
    replace: "\n",
    tests: ["test/corpus.test.ts", "-t", "^fn_boxed_once_drops.rs$"],
  },
  {
    name: "fn-once-dyn-consumed-again",
    breaks:
      "a `dyn FnOnce` made a `dyn FnOnce` again, returned or given on, is wrapped to drop as its call returns, dropping what it took twice (#409)",
    file: "src/lower/traits/dyns.rs",
    find: "                && !matches!(function.kind(), ty::Dynamic(of, ..) if crate::lower::recognition::is_fn_once_dyn(self.tcx, of));",
    replace: ";",
    tests: ["test/corpus.test.ts", "-t", "^dyn_fn_once_drops_itself.rs$"],
  },
  {
    name: "fn-dyn-made-once-always-wrapped",
    breaks:
      "a `dyn Fn` made a `dyn FnOnce` is wrapped to drop as it's called, `$droppingOnce(f, ..)`, where the program makes no `dyn Fn` of what has something to drop, not `queueMicrotask(callback)` (#409)",
    file: "src/lower/traits/dyns.rs",
    find: "                ty::Dynamic(..) => facts.fn_dyns_drop,\n",
    replace: "                ty::Dynamic(..) => true,\n",
    tests: ["test/webapi-bindings.test.ts", "-t", "calls once moves"],
  },
];
