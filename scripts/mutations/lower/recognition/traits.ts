// Mutations of src/lower/recognition/traits.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "union-function-member-unseen",
    breaks:
      "a binding's union whose `Function` member is any function called once, `js::set_timeout`'s `H: IntoTimerHandler`, isn't called once, so a closure rustc infers `Fn` given to it, `move || { let s = &second; .. }`, isn't one whose call drops what it took, and is refused (ADR 1020)",
    file: "src/lower/recognition/traits.rs",
    find: "                (tcx.fn_trait_kind_from_def_id(bound.def_id())).or_else(|| member_function_kind(tcx, bound.def_id()))\n",
    replace: "                tcx.fn_trait_kind_from_def_id(bound.def_id())\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^called_once_closure_drops_after.rs$",
    ],
  },
  {
    name: "union-function-member-of-any-trait",
    breaks:
      "a trait of the program's implemented for any function is taken for a binding's union, so a binding's parameter of it is a once-callback, `keepAny(shown)` of `|x| shown(x)`, which JS may give more, where the trait says nothing of how JS calls it (ADR 1020)",
    file: "src/lower/recognition/traits.rs",
    find: "    tcx.get_attrs_by_path(trait_id, &union).next()?;\n",
    replace: "",
    tests: [
      "test/lowering-passed-on.test.ts",
      "-t",
      "FnOnce callback that only passes its call on",
    ],
  },
  {
    name: "equated-bounds-unseen",
    breaks:
      "a type parameter the where-clauses make an iterator's associated type isn't bound as an iterator: its `&mut` a cell, `(x0) => $next(x0.value)`, where it's the iterator, and std's `Flatten::try_fold` refused (ADR 0924)",
    file: "src/lower/recognition/traits.rs",
    find: "        let clauses = clauses.chain(declared.into_iter().flatten()).chain(equated);\n",
    replace:
      "        let clauses = clauses.chain(declared.into_iter().flatten()).chain(equated.filter(|_| false));\n",
    snapshots: true,
    tests: ["test/corpus.test.ts", "-t", "^equated_iterator_parameter.rs$"],
  },
  {
    name: "into-a-bound-target-unresolved",
    breaks:
      "`x.into()` of a type parameter's target, `U: From<T>`, is refused, its `From` no impl to resolve (#321)",
    file: "src/lower/recognition/traits.rs",
    find: "            _ if known_by_bounds(args.type_at(0)) => method,\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^into_a_bound_target.rs$"],
  },
  {
    name: "trait-of-no-items-impl",
    breaks:
      "an impl of a trait of no items, `unsafe impl Sync`, `impl FusedIterator`, is rejected, though it runs nothing",
    file: "src/lower/recognition/traits.rs",
    find: "        || (tcx.associated_item_def_ids(id).iter()).all(|&item| matches!(tcx.def_kind(item), DefKind::AssocTy))\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "marker_traits|fused_iterator"],
  },
  {
    name: "library-trait-no-dictionary",
    breaks:
      "a library's trait isn't one whose impls are dictionaries, so a call of its generic function passes none",
    file: "src/lower/recognition/traits.rs",
    find: "        || foreign.in_library(id)\n",
    replace: "        || false\n",
    tests: ["test/crates.test.ts", "-t", "two crates: dict"],
  },
  {
    name: "generic-operators-refused",
    breaks: "`a + b` of a `T: Add` has no dictionary, and is refused",
    file: "src/lower/recognition/traits.rs",
    find: "        || value_operator(tcx, id).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "generic_operators"],
  },
  {
    name: "generic-into-refused",
    breaks:
      "`name.into()` of an `impl Into<String>` has no dictionary, and is refused",
    file: "src/lower/recognition/traits.rs",
    find: "        || tcx.is_diagnostic_item(sym::Into, id)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "generic_into"],
  },
  {
    name: "generic-bitwise-operators",
    breaks:
      "`a | b` of a `T: BitOr` in generic code is refused, where `a + b` of a `T: Add` isn't",
    file: "src/lower/recognition/traits.rs",
    find: "        (LangItem::BitOr, Ok(BinOp::BitOr)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "generic_bitwise_operators"],
  },
  {
    name: "to-string-not-operational",
    breaks:
      "a `T: ToString` bound is given no dictionary, and `x.to_string()` asks for a `Display`",
    file: "src/lower/recognition/traits.rs",
    find: '        || tcx.is_diagnostic_item(Symbol::intern("ToString"), id)\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "evidence_paths"],
  },
  {
    name: "projection-bounds-undeclared",
    breaks:
      "an associated type's bound in its trait, `type Iter: Iterator`, isn't seen: a `&mut` to one is boxed, and what steps it is given the box",
    file: "src/lower/recognition/traits.rs",
    find: "            ) => Some(self.tcx.item_bounds(*def_id).skip_binder()),\n",
    replace: "            ) => Some(ty::List::empty()),\n",
    tests: ["test/corpus.test.ts", "-t", "generic_associated_types"],
  },
  {
    name: "user-from-str-refused",
    breaks:
      "a type's own `impl FromStr` is an error, and `T: FromStr` in generic code",
    file: "src/lower/recognition/traits.rs",
    find: "        || is_from_str(tcx, id)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "user_from_str|generic_from_str"],
  },
  {
    name: "delegated-sum-unresolved",
    breaks: "`sum()` into a type of the crate's isn't its `Sum`",
    file: "src/lower/recognition/traits.rs",
    find: '            "sum" => "sum",\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^user_collections.rs$"],
    snapshots: true,
  },
  {
    name: "delegated-own-param-dropped",
    breaks:
      "`collect()` into a collection of the crate's resolves its `from_iter` without the iterator, which doesn't resolve",
    file: "src/lower/recognition/traits.rs",
    find: "args.types().take(own)",
    replace: "args.types().take(0)",
    tests: ["test/corpus.test.ts", "-t", "user_collections"],
  },
  {
    name: "as-ref-not-dictionary",
    breaks:
      "`s.as_ref()` of a generic `S: AsRef<str>`, and a type's own `impl AsRef`, are errors",
    file: "src/lower/recognition/traits.rs",
    find: "        || is_std_def(tcx, id, StdItem::AsRef)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "generic_as_ref"],
  },
  {
    name: "iterator-extensions-refused",
    breaks: "a user `DoubleEndedIterator` or `ExactSizeIterator` is an error",
    file: "src/lower/recognition/traits.rs",
    find: "        || is_iterator_extension(tcx, id)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "double_ended_iterators"],
  },
  {
    name: "user-fmt-write-refused",
    breaks: "a writer of the crate's own, `impl fmt::Write`, is an error",
    file: "src/lower/recognition/traits.rs",
    find: '        || tcx.is_diagnostic_item(Symbol::intern("FmtWrite"), id)\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "user_fmt_write|generic_writers"],
  },
  {
    name: "user-borrow-refused",
    breaks: "a type's own `impl Borrow` is an error",
    file: "src/lower/recognition/traits.rs",
    find: "        || is_std_def(tcx, id, StdItem::Borrow)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "generic_borrow"],
  },
  {
    name: "user-hash-refused",
    breaks: "a type's own `impl Hash` is an error",
    file: "src/lower/recognition/traits.rs",
    find: "        || is_hashing(tcx, id)\n        // `items.sum()` into an `S: Sum<A>`",
    replace: "        // `items.sum()` into an `S: Sum<A>`",
    tests: ["test/corpus.test.ts", "-t", "user_hash"],
  },
  {
    name: "user-borrow-mut-refused",
    breaks: "a type's own `impl BorrowMut` is an error",
    file: "src/lower/recognition/traits.rs",
    find: "        || is_std_def(tcx, id, StdItem::BorrowMut)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "user_as_mut"],
  },
  {
    name: "other-fmt-traits-not-dictionaries",
    breaks:
      "a user `LowerHex` or `Pointer` is an error, and `{:x}` of a generic `T`",
    file: "src/lower/recognition/traits.rs",
    find: "        // `T: LowerHex` or `T: Pointer` (ADR 0174).\n        || is_other_fmt_trait(tcx, id)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "user_fmt_traits|generic_fmt_traits"],
  },
  {
    name: "number-lt-in-default-unknown",
    breaks:
      "`self < Self::zero()` in a default copied into an `f64`'s impl is an error",
    file: "src/lower/recognition/traits.rs",
    find: '!matches!(name.as_str(), "partial_cmp" | "lt" | "le" | "gt" | "ge")',
    replace: '!matches!(name.as_str(), "partial_cmp")',
    tests: ["test/corpus.test.ts", "-t", "trait_default_bounds"],
  },
  {
    name: "projection-not-unknown",
    breaks:
      "`drain`'s `Vec<S::Item>` is refused as a type, and std's `I::Item` too",
    file: "src/lower/recognition/traits.rs",
    find: "            ) => self\n                .tcx\n                .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))\n",
    replace:
      "            ) if false => self\n                .tcx\n                .try_normalize_erasing_regions(typing_env, ty::Unnormalized::new_wip(ty))\n",
    tests: ["test/corpus.test.ts", "-t", "associated_types"],
  },
  {
    name: "type-only-trait-refused",
    breaks:
      "an impl of a trait of types only, react's `Lifetimes`, which runs nothing, is refused as a user implementation",
    file: "src/lower/recognition/traits.rs",
    find: "(tcx.associated_item_def_ids(id).iter()).all(|&item| matches!(tcx.def_kind(item), DefKind::AssocTy))",
    replace: "tcx.associated_item_def_ids(id).is_empty()",
    tests: [
      "test/jsx-evaluation-order.test.ts",
      "-t",
      "thread_local component takes props",
    ],
  },
  {
    name: "throwing-call-uncaught",
    breaks:
      "a JS function Rust says gives a `Result` isn't run in a `try`, so what it throws is thrown, not an `Err` its caller can match or `?`",
    file: "src/lower/recognition/traits.rs",
    find: "        if self.is_std_type(output, StdItem::Result) {\n            Catching::Result",
    replace:
      "        if false && self.is_std_type(output, StdItem::Result) {\n            Catching::Result",
    tests: [
      "test/compiler.test.ts",
      "-t",
      "a throwing JS call is a Result, and \\? returns early",
    ],
  },
  {
    name: "binding-result-uncaught",
    breaks:
      "a binding that returns a `Result`, `d.to_iso_string()`, isn't run in a `try`: what it throws escapes, and what it returns is taken for a `Result`",
    file: "src/lower/recognition/traits.rs",
    find: "        if self.is_std_type(output, StdItem::Result) {\n            Catching::Result",
    replace:
      "        if false && self.is_std_type(output, StdItem::Result) {\n            Catching::Result",
    tests: [
      "test/js-builtins.test.ts",
      "-t",
      "a Date is JS's, its members methods",
    ],
  },
  {
    name: "catching-generic-signature",
    breaks:
      "whether a binding may throw is its generic signature's: `stringify_with` of an `Unknown` isn't in a `try`",
    file: "src/lower/recognition/traits.rs",
    find: "        let output = sig\n            .instantiate(self.tcx, args)\n",
    replace: "        let output = sig\n            .instantiate_identity()\n",
    tests: ["test/bindings.test.ts", "-t", "react.dev's Console"],
  },
  {
    name: "own-clone-from-of-std",
    breaks: "a type's own `clone_from` is taken for std's",
    file: "src/lower/recognition/traits.rs",
    find: "            Ok(ImplSource::UserDefined(imp)) if !self.is_std(imp.impl_def_id) && self.has_clone_from(imp.impl_def_id))",
    replace: "            Ok(ImplSource::UserDefined(imp)) if false)",
    tests: ["test/corpus.test.ts", "-t", "clone_from_slice_own"],
  },
  {
    name: "ties-always-unseen",
    breaks: "an unstable sort of a type's own `Ord` is JS's stable sort",
    file: "src/lower/recognition/traits.rs",
    find: "            _ => {\n                !ty.walk()\n",
    replace: "            _ => true || {\n                !ty.walk()\n",
    tests: ["test/corpus.test.ts", "-t", "sort_unstable_ties"],
  },
  {
    name: "wrapping-const-unknown",
    breaks: "a constant `Wrapping(x)` is refused",
    file: "src/lower/recognition/traits.rs",
    find: '        || tcx.is_diagnostic_item(Symbol::intern("Wrapping"), id)\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "wrapping_type"],
  },
  {
    name: "cmp-max-unknown",
    breaks: "`cmp::max(a, b)` of what isn't a number is refused",
    file: "src/lower/recognition/traits.rs",
    find: '        "std::cmp::max" => Some(true),\n',
    replace: '        "std::cmp::max" => None,\n',
    tests: ["test/corpus.test.ts", "-t", "std_methods"],
  },
  {
    name: "io-write-not-operational",
    breaks: "a generic `W: io::Write` is given no dictionary, and refused",
    file: "src/lower/recognition/traits.rs",
    find: "        || is_std_def(tcx, id, StdItem::IoWrite)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^io_writers.rs$"],
  },
  {
    name: "io-write-all-not-given",
    breaks:
      "generic code's `out.write_all(bytes)` isn't its dictionary's, and is refused",
    file: "src/lower/recognition/traits.rs",
    find: '        || is_std_method(tcx, id, StdItem::IoWrite, "write_all")\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^io_writers.rs$"],
  },
  {
    name: "deref-not-operational",
    breaks: "a `P: Deref` in generic code is given no dictionary, and refused",
    file: "src/lower/recognition/traits.rs",
    find: "        || tcx.is_lang_item(id, LangItem::Deref)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_deref.rs$"],
  },
  {
    name: "assign-operator-not-operational",
    breaks:
      "a `T: AddAssign` in generic code is given no dictionary, and refused",
    file: "src/lower/recognition/traits.rs",
    find: "        // `a += b` of a `T: AddAssign`: its dictionary's `add_assign` (ADR 0396).\n        || assign_operator(tcx, id).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_compound_assign.rs$"],
  },
  {
    name: "index-impl-refused",
    breaks:
      "the crate's own `impl Index` is refused, not an operator it may implement",
    file: "src/lower/recognition/traits.rs",
    find: "        || [LangItem::Index, LangItem::IndexMut]\n",
    replace: "        || [LangItem::IndexMut]\n",
    tests: ["test/corpus.test.ts", "-t", "^mut_ref_kept.rs$"],
  },
  {
    name: "range-bounds-unoperational",
    breaks:
      "`range.start_bound()` in generic code is refused, though a range's dictionary gives it",
    file: "src/lower/recognition/traits.rs",
    find: "        || is_std_def(tcx, id, StdItem::RangeBounds)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^range_bounds_generic.rs$"],
  },
  {
    name: "user-future-refused",
    breaks: "a type's own `impl Future`, either's, is an error",
    file: "src/lower/recognition/traits.rs",
    find: "    tcx.is_lang_item(id, LangItem::Future)\n        || matches!(\n",
    replace: "    false\n        || matches!(\n",
    tests: ["test/corpus.test.ts", "-t", "^unlowered_impls.rs$"],
  },
  {
    name: "user-read-refused",
    breaks: "a type's own `impl io::Read` is an error",
    file: "src/lower/recognition/traits.rs",
    find: '            "std::io::Read" | "std::io::BufRead" | "std::io::Seek"\n',
    replace: '            "std::io::BufRead" | "std::io::Seek"\n',
    tests: ["test/corpus.test.ts", "-t", "^unlowered_impls.rs$"],
  },
  {
    name: "user-buf-read-refused",
    breaks: "a type's own `impl BufRead` is an error",
    file: "src/lower/recognition/traits.rs",
    find: '            "std::io::Read" | "std::io::BufRead" | "std::io::Seek"\n',
    replace: '            "std::io::Read" | "std::io::Seek"\n',
    tests: ["test/corpus.test.ts", "-t", "^unlowered_impls.rs$"],
  },
  {
    name: "user-seek-refused",
    breaks: "a type's own `impl Seek` is an error",
    file: "src/lower/recognition/traits.rs",
    find: '            "std::io::Read" | "std::io::BufRead" | "std::io::Seek"\n',
    replace: '            "std::io::Read" | "std::io::BufRead"\n',
    tests: ["test/corpus.test.ts", "-t", "^unlowered_impls.rs$"],
  },
  {
    name: "dyn-display-refused",
    breaks: "a `dyn Display` isn't a pair, and is refused",
    file: "src/lower/recognition/traits.rs",
    find: "            StdItem::Display,\n            StdItem::ToString,\n",
    replace: "            StdItem::ToString,\n",
    tests: ["test/corpus.test.ts", "-t", "dyn_display"],
  },
  {
    name: "any-not-pair",
    breaks: "a `dyn Any` is an error, not a pair",
    file: "src/lower/recognition/traits.rs",
    find: "            || any_trait(self.tcx) == Some(id)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_any.rs"],
  },
  {
    name: "dyn-to-string-refused",
    breaks: "a `dyn ToString` is no value rust-js makes",
    file: "src/lower/recognition/traits.rs",
    find: "            StdItem::ToString,\n            StdItem::Error,\n",
    replace: "            StdItem::Error,\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_to_string.rs$"],
  },
  {
    name: "auto-trait-dyn-unpaired",
    breaks:
      "a `dyn` of no trait but auto traits, `dyn Send`, isn't a pair, so a `Box<dyn Send>` of a value with a destructor is refused",
    file: "src/lower/recognition/traits.rs",
    find: "        predicates.principal_def_id().is_none_or(|id| self.is_pair_trait(id))\n",
    replace:
      "        predicates.principal_def_id().is_some_and(|id| self.is_pair_trait(id))\n",
    tests: ["test/corpus.test.ts", "-t", "^dyn_auto_traits.rs$"],
  },
  {
    name: "dyn-fmt-write-refused",
    breaks: "a `&mut dyn fmt::Write` is refused, a function of any writer",
    file: "src/lower/recognition/traits.rs",
    find: "            StdItem::IoWrite,\n            StdItem::FmtWrite,\n        ]\n",
    replace: "            StdItem::IoWrite,\n        ]\n",
    tests: ["test/corpus.test.ts", "-t", "^fmt_write_function.rs$"],
  },
  {
    name: "as-mut-not-operational",
    breaks:
      "a `T: AsMut<str>` in generic code is given no dictionary, and refused",
    file: "src/lower/recognition/traits.rs",
    find: "        || is_std_def(tcx, id, StdItem::AsMut)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^as_mut_generic.rs$"],
  },
  {
    name: "error-defaults-not-in-dictionary",
    breaks:
      "an `Error`'s `description` and `cause`, called in generic code, aren't in its dictionary, and are refused",
    file: "src/lower/recognition/traits.rs",
    find: "        || error_default(tcx, id).is_some()\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^error_description_cause.rs$"],
  },
  {
    name: "deref-mut-not-operational",
    breaks:
      "a `P: DerefMut` in generic code is given no dictionary, and refused",
    file: "src/lower/recognition/traits.rs",
    find: "        || tcx.is_lang_item(id, LangItem::DerefMut)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^generic_deref_mut.rs$"],
  },
  {
    name: "deref-mut-holds-no-supertraits",
    breaks:
      "a `P: DerefMut`'s `Deref` isn't looked for in its dictionary, and generic code that reads one is refused",
    file: "src/lower/recognition/traits.rs",
    find: "            || self.tcx.is_lang_item(id, LangItem::DerefMut)\n            || is_std_def(self.tcx, id, StdItem::BorrowMut)\n",
    replace: "            || is_std_def(self.tcx, id, StdItem::BorrowMut)\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_deref_mut.rs$"],
  },
  {
    name: "into-iter-not-operational",
    breaks:
      "`IntoIterator` has no dictionaries, and a `for<'a> &'a mut C: IntoIterator` bound's `for x in c` is refused",
    file: "src/lower/recognition/traits.rs",
    find: "        // dictionary's `into_iter` (ADR 0457).\n        || tcx.is_diagnostic_item(sym::IntoIterator, id)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_of_ref_param.rs$"],
  },
  {
    name: "serde-visitor-unimplementable",
    breaks: "a hand-written `Visitor` is refused",
    file: "src/lower/recognition/traits.rs",
    find: "        || serde_visitor(tcx, id)\n",
    replace: "",
    tests: ["test/serde.test.ts", "-t", "hand-written Visitor"],
  },
  {
    name: "hash-slice-not-in-dictionary",
    breaks:
      "a `Hash`'s dictionary has no `hash_slice`, which a slice of its type calls",
    file: "src/lower/recognition/traits.rs",
    find: '        || is_std_method(tcx, id, StdItem::Hash, "hash_slice")\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^hash_values.rs$"],
  },
  {
    name: "dyn-hasher-no-pair",
    breaks: "a `dyn Hasher` is refused, as no pair",
    file: "src/lower/recognition/traits.rs",
    find: "            || is_hasher(self.tcx, id)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^hash_hashers.rs$"],
  },
  {
    name: "function-kind-weakest",
    breaks:
      "an `F: FnMut` is taken for one called by `FnOnce`, whose body drops what it holds, and left undropped",
    file: "src/lower/recognition/traits.rs",
    find: "            .reduce(|a, b| if a.extends(b) { a } else { b }),\n",
    replace: "            .reduce(|a, b| if a.extends(b) { b } else { a }),\n",
    tests: ["test/corpus.test.ts", "-t", "^given_function_drops.rs$"],
  },
  {
    name: "sum-bound-ungiven",
    breaks:
      "a generic function's `S: Sum<A>` is given no dictionary, and its `sum()` refused",
    file: "src/lower/recognition/traits.rs",
    find: "        || is_sum_or_product(tcx, id)\n}\n",
    replace: "}\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_accumulations.rs$"],
  },
  {
    name: "from-bound-ungiven",
    breaks:
      "a generic function's `W: From<U>` is given no dictionary, and `W::from(u)` refused",
    file: "src/lower/recognition/traits.rs",
    find: "        || tcx.is_diagnostic_item(sym::From, id)\n        // A writer of the crate's own",
    replace: "        // A writer of the crate's own",
    tests: ["test/corpus.test.ts", "-t", "^generic_from.rs$"],
  },
  {
    name: "borrow-mut-bound-ungiven",
    breaks:
      "a `BorrowMut` supertrait gives its trait's dictionary no entry, so `self.borrow()` of a trait's own default finds no `Borrow`",
    file: "src/lower/recognition/traits.rs",
    find: "        || is_std_def(tcx, id, StdItem::Borrow)\n        || is_std_def(tcx, id, StdItem::BorrowMut)\n",
    replace:
      "        || is_std_def(tcx, id, StdItem::Borrow)\n        || serde_visitor(tcx, id) && false\n",
    tests: ["test/corpus.test.ts", "-t", "^borrow_mut_supertrait.rs$"],
  },
  {
    name: "returned-closure-carries-nothing",
    breaks:
      "a closure its maker gives back is taken for one that carries no drop, and refused where it's dropped",
    file: "src/lower/recognition/traits.rs",
    find: "    let maker = tcx.typeck_root_def_id(closure);\n    if !matches!(tcx.def_kind(maker), DefKind::Fn | DefKind::AssocFn) {\n",
    replace:
      "    let maker = tcx.typeck_root_def_id(closure);\n    if true || !matches!(tcx.def_kind(maker), DefKind::Fn | DefKind::AssocFn) {\n",
    tests: ["test/corpus.test.ts", "-t", "^returned_closure_drops.rs$"],
  },
  {
    name: "borrow-mut-holds-no-borrow",
    breaks:
      "a `BorrowMut` dictionary is taken to hold no `Borrow`, so `self.borrow()` of a trait's own default through it is refused",
    file: "src/lower/recognition/traits.rs",
    find: "            || is_std_def(self.tcx, id, StdItem::BorrowMut)\n            // `Try`'s its `FromResidual` (ADR 0913).\n",
    replace: "            // `Try`'s its `FromResidual` (ADR 0913).\n",
    tests: ["test/corpus.test.ts", "-t", "^borrow_mut_supertrait.rs$"],
  },
  {
    name: "known-by-bounds-not-associated",
    breaks:
      "an associated type, `T::Buffer`, isn't known by its bounds, and its `as_mut()` is refused",
    file: "src/lower/recognition/traits.rs",
    find: "    matches!(ty.kind(), ty::Param(_) | ty::Alias(..))\n}",
    replace: "    matches!(ty.kind(), ty::Param(_))\n}",
    tests: ["test/corpus.test.ts", "-t", "^associated_type_as_mut.rs$"],
  },
  {
    name: "into-future-unimplementable",
    breaks: "an `IntoFuture` of the crate's is refused",
    file: "src/lower/recognition/traits.rs",
    find: '        || tcx.is_diagnostic_item(Symbol::intern("IntoFuture"), id)\n        // Never lowered',
    replace: "        // Never lowered",
    tests: ["test/snapshots.test.ts", "-t", "async"],
  },
  {
    name: "generic-result-taken-to-throw",
    breaks:
      "a binding declared to give a `T`, given a `Result`, runs in a `$try`, its `Result` wrapped in another `Ok`",
    file: "src/lower/recognition/traits.rs",
    find: "        if given(declared)\n",
    replace: "        if false\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "-t",
      "generic binding given a Result",
    ],
  },
  {
    name: "generic-promise-result-settled",
    breaks:
      "`js::promise` of an `async fn` that gives a `Result` is settled, its `Result` wrapped in another `Ok`",
    file: "src/lower/recognition/traits.rs",
    find: "                && inner.types().next().is_some_and(given))\n",
    replace:
      "                && inner.types().next().is_some_and(|_| false))\n",
    tests: [
      "test/lowering-bindings.test.ts",
      "-t",
      "generic binding given a Result",
    ],
  },
  {
    name: "assert-unwind-safe-uncalled",
    breaks:
      "a closure given to `catch_unwind` through `AssertUnwindSafe` isn't called as what it holds, and never drops what it took (#196)",
    file: "src/lower/recognition/traits.rs",
    find: "        ty::Adt(adt, args) if super::wrappers::is_assert_unwind_safe(tcx, adt.did()) => {\n",
    replace:
      "        ty::Adt(adt, args) if false && super::wrappers::is_assert_unwind_safe(tcx, adt.did()) => {\n",
    tests: ["test/corpus.test.ts", "-t", "^catch_unwind_assert_drops.rs$"],
  },
];
