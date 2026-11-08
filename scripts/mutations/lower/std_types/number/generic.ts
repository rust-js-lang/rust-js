// Mutations of src/lower/std_types/number/generic.rs (ADR 0093).
import type { Mutation } from "../../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "min-chain-nested",
    breaks:
      "`a.min(b).min(c)` is `Math.min(Math.min(a, b), c)`, not `Math.min(a, b, c)`",
    file: "src/lower/std_types/number/generic.rs",
    find: "                    js::ExprKind::Call(inner, args) if math && js::same_path(&inner, &callee) => args,\n",
    replace:
      "                    js::ExprKind::Call(inner, args) if false && js::same_path(&inner, &callee) => args,\n",
    tests: ["test/corpus.test.ts", "-t", "^min_max_chains.rs$"],
    snapshots: true,
  },
  {
    name: "min-chain-of-a-helper-flattened",
    breaks:
      "`a.min(b).min(c)` of floats splices `$f64Min`'s arguments into another `$f64Min`, which takes two",
    file: "src/lower/std_types/number/generic.rs",
    find: "                    js::ExprKind::Call(inner, args) if math && js::same_path(&inner, &callee) => args,\n",
    replace:
      "                    js::ExprKind::Call(inner, args) if js::same_path(&inner, &callee) => args,\n",
    tests: ["test/corpus.test.ts", "-t", "^min_max_chains.rs$"],
  },
  {
    name: "array-of-slice-aliased",
    breaks:
      "`<[T; N]>::try_from(&v[..])` is `v`, which a write to `v` after changes",
    file: "src/lower/std_types/number/generic.rs",
    find: "                let value = match target.is_ref() || owned || (fresh && !self.needs_clone(item)) {",
    replace: "                let value = match true {",
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "array made of a whole slice",
    ],
  },
  {
    name: "array-of-part-copied-twice",
    breaks: "`<[T; N]>::try_from(&v[1..3])` copies `$slice`'s copy again",
    file: "src/lower/std_types/number/generic.rs",
    find: "                let value = match target.is_ref() || owned || (fresh && !self.needs_clone(item)) {",
    replace: "                let value = match target.is_ref() || owned {",
    tests: ["test/corpus.test.ts", "-t", "slice_to_array"],
    snapshots: true,
  },
  {
    name: "size-align-swap",
    breaks: "`align_of` is the type's size",
    file: "src/lower/std_types/number/generic.rs",
    find: "        let bytes = if fact == TypeFact::Align {\n            layout.align.abi.bytes()",
    replace:
      "        let bytes = if fact == TypeFact::Align {\n            layout.size.bytes()",
    tests: ["test/corpus.test.ts", "-t", "size_of\\.rs"],
  },
  {
    name: "type-name-unerased",
    breaks:
      "`type_name` names the type as written, `Vec<Option<&str>>`, not rustc's paths",
    file: "src/lower/std_types/number/generic.rs",
    find: "            _ => rustc_const_eval::util::type_name(self.tcx, of),",
    replace: "            _ => of.to_string(),",
    tests: ["test/corpus.test.ts", "-t", "std_odds"],
  },
  {
    name: "duration-subsec-whole",
    breaks:
      "`subsec_nanos()` is all of a `Duration`'s nanoseconds, not those under a second",
    file: "src/lower/std_types/number/generic.rs",
    find: "                        let part = Expr::bin(Op::Rem, arg(), nanos(1_000_000_000));",
    replace: "                        let part = arg();",
    tests: ["test/corpus.test.ts", "-t", "duration"],
  },
  {
    name: "slice-to-array-any-length",
    breaks: "a slice into an array of another length is `Ok`",
    file: "src/lower/std_types/number/generic.rs",
    find: '                let fits = Expr::bin(Op::Eq, Expr::member(slice.clone(), "length"), Expr::int(len as i128));',
    replace: "                let fits = Expr::bool(true || len > 0);",
    tests: ["test/corpus.test.ts", "-t", "slice_to_array"],
  },
  {
    name: "non-zero-try-from-kind",
    breaks:
      "`NonZeroU8::try_from(0u8)`'s error is its kind, `Zero`, which `{:?}` can't show, where each parse error is its message",
    file: "src/lower/std_types/number/generic.rs",
    find: "number would be zero for non-zero type",
    replace: "Zero",
    tests: ["test/corpus.test.ts", "-t", "nonzero"],
  },
  {
    name: "needs-drop-inverted",
    breaks: "`mem::needs_drop::<T>()` answers the opposite of rustc",
    file: "src/lower/std_types/number/generic.rs",
    find: "Expr::bool(of.needs_drop(self.tcx, self.typing_env))",
    replace: "Expr::bool(!of.needs_drop(self.tcx, self.typing_env))",
    tests: ["test/corpus.test.ts", "-t", "^needs_drop.rs$"],
  },
  {
    name: "assoc-fact-not-read",
    breaks:
      "`size_of::<A::Item>()` isn't read from `A: Array`'s dictionary, and is refused",
    file: "src/lower/std_types/number/generic.rs",
    find: "                .is_some_and(|facts| facts.iter().any(|&(_, f)| f == fact))\n",
    replace: "                .is_some_and(|_| false)\n",
    tests: ["test/corpus.test.ts", "-t", "^associated_type_facts.rs$"],
  },
  {
    name: "generic-type-name-refused",
    breaks: "`type_name` of a type that holds a type parameter is an error",
    file: "src/lower/std_types/number/generic.rs",
    find: "        if fact == TypeFact::Name && of.has_param() {\n",
    replace: "        if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^type_names_generic.rs$"],
  },
  {
    name: "checked-ilog-unchecked",
    breaks: "`checked_ilog10()` is `ilog10()`, which panics of zero",
    file: "src/lower/std_types/number/generic.rs",
    find: '    if checked {\n        given.push(Expr::bool(true));\n    }\n    Expr::call_helper("$ilog", given)\n',
    replace: '    let _ = checked;\n    Expr::call_helper("$ilog", given)\n',
    tests: ["test/corpus.test.ts", "-t", "^number_methods.rs$"],
  },
  {
    name: "sort-fact-one-kind",
    breaks:
      "every caller gives `TSort` the small sort of a `Copy` item of 8 bytes, which leaves a wider one's ties where std's doesn't",
    file: "src/lower/std_types/number/generic.rs",
    find: "            return Ok(Expr::str(self.sort_kind(of, span)?));\n",
    replace: '            return Ok(Expr::str("network"));\n',
    tests: ["test/corpus.test.ts", "-t", "^generic_unstable_sorts.rs$"],
  },
  {
    name: "fused-fact-unanswered",
    breaks: "a type parameter's `Fused` no caller says is refused, not `false`",
    file: "src/lower/std_types/number/generic.rs",
    find: "                return Ok(self.fused_fact(param.index).unwrap_or_else(|| Expr::bool(false)));\n",
    replace:
      '                return self.fused_fact(param.index).ok_or_else(|| self.unsupported(span, "Fused"));\n',
    tests: ["test/corpus.test.ts", "-t", "^fuse_size_hint.rs$"],
  },
  {
    name: "vec-to-array-err-empty",
    breaks:
      "a `Vec` refused as an array gives back nothing in its `Err`, not the `Vec`",
    file: "src/lower/std_types/number/generic.rs",
    find: "                    true => slice.clone(),\n                    false => Expr::undefined(),",
    replace:
      "                    true => Expr::undefined(),\n                    false => Expr::undefined(),",
    tests: ["test/corpus.test.ts", "-t", "^vec_into_array.rs$"],
  },
  {
    name: "vec-to-array-copied",
    breaks:
      "a `Vec` moved into an array is copied first, `v.slice()`, as nothing else has it",
    file: "src/lower/std_types/number/generic.rs",
    find: "                let value = match target.is_ref() || owned || (fresh && !self.needs_clone(item)) {",
    replace:
      "                let value = match target.is_ref() || (fresh && !self.needs_clone(item)) {",
    tests: ["test/corpus.test.ts", "-t", "^vec_into_array.rs$"],
    snapshots: true,
  },
  {
    name: "slice-to-array-items-shared",
    breaks:
      "an array made of a slice of `Copy` structs, `try_into()`, holds the slice's objects, so changing one changes the slice",
    file: "src/lower/std_types/number/generic.rs",
    find: "                    false => self.clone_items(slice.clone(), item, span)?,",
    replace:
      '                    false => Expr::call(Expr::member(slice.clone(), "slice"), vec![]),',
    tests: ["test/corpus.test.ts", "-t", "^copied_structs_apart.rs$"],
  },
  {
    name: "one-shot-never",
    breaks:
      "no type is a `OneShot`, so a held flatten of `Option`s steps by the general impl, its hint `(0, None)`",
    file: "src/lower/std_types/number/generic.rs",
    find: "            }\n            return Ok(Expr::bool(holds));\n        }\n        // `N` of an array",
    replace:
      "            }\n            return Ok(Expr::bool(holds && fact != TypeFact::OneShot));\n        }\n        // `N` of an array",
    tests: ["test/corpus.test.ts", "-t", "^std_flatten.rs$"],
  },
  {
    name: "tried-int-below-unchecked",
    breaks:
      "`usize::try_from(-1)` only tested is `Ok`, as the bound below wasn't tested (#366)",
    file: "src/lower/std_types/number/generic.rs",
    find: "    let below = (from_lo < lo).then(|| compared(Op::Lt, lo));\n",
    replace:
      "    let below = (from_lo < lo && false).then(|| compared(Op::Lt, lo));\n",
    tests: ["test/corpus.test.ts", "-t", "^try_from_int_tested.rs$"],
  },
  {
    name: "tried-int-above-unchecked",
    breaks:
      "`u8::try_from(256)` only tested is `Ok`, as the bound above wasn't tested (#366)",
    file: "src/lower/std_types/number/generic.rs",
    find: "    let above = (from_hi > hi).then(|| compared(Op::Gt, hi as i128));\n",
    replace:
      "    let above = (from_hi > hi && false).then(|| compared(Op::Gt, hi as i128));\n",
    tests: ["test/corpus.test.ts", "-t", "^try_from_int_tested.rs$"],
  },
  {
    name: "tried-int-not-made-big",
    breaks:
      "`i64::try_from(n)` of a `u32` only tested is the number, not its BigInt, so `wide + 1n` throws (#366)",
    file: "src/lower/std_types/number/generic.rs",
    find: '        (false, true) => Expr::call(Expr::var("BigInt"), vec![n]),\n',
    replace: "        (false, true) => n,\n",
    tests: ["test/corpus.test.ts", "-t", "^try_from_int_tested.rs$"],
  },
  {
    name: "in-place-items-given-none",
    breaks:
      "a caller gives generic code of a `Vec`'s `IntoIter` no buffer std collects it in, in place, `undefined`, so a panic stopping its collect drops what's left first (#672)",
    file: "src/lower/std_types/number/generic.rs",
    find: "                Some(InPlaceSource::Items(item)) => Ok(Expr::array(vec![\n",
    replace: "                Some(InPlaceSource::Items(item)) if false => Ok(Expr::array(vec![\n",
    tests: ["test/corpus.test.ts", "-t", "^generic_collect_drops_rest_first.rs$"],
  },
  {
    name: "built-layout-given-unread",
    breaks:
      "a type built of a function's type parameters whose layout its caller gives, `Held<J>`, is measured as if rustc knew it, and refused, `size_of` of a type parameter (#737)",
    file: "src/lower/std_types/number/generic.rs",
    find: "        if (matches!(of.kind(), ty::Alias(..)) || !matches!(of.kind(), ty::Param(_)) && of.has_param())\n",
    replace: "        if matches!(of.kind(), ty::Alias(..))\n",
    tests: ["test/corpus.test.ts", "-t", "^in_place_collect_of_a_built_type.rs$"],
  },
  {
    name: "const-size-of-built-type-asked",
    breaks:
      "the constant size of a type built from a type parameter that's no array, `&Vec<T>`, is asked of a caller who can't give it, so generic code's `rows.iter().flatten()` is refused in the build of std's structs (#813)",
    file: "src/lower/std_types/number/generic.rs",
    find: "            TypeFact::ConstSize => crate::lower::recognition::known_by_bounds(of.peel_refs()),\n",
    replace: "            TypeFact::ConstSize => of.has_param() || matches!(of.kind(), ty::Alias(..)),\n",
    tests: ["test/iterator-forms.test.ts", "-t", "generic_flatten_of_vecs"],
  },
  {
    name: "const-size-of-a-const-length-none",
    breaks:
      "an array of a const parameter's length is said to have no constant size, so std's flatten of them hints no upper bound, where Rust's is the rows times `N` (#813)",
    file: "src/lower/std_types/number/generic.rs",
    find: "                    None => self.const_arg(len, span),\n",
    replace: "                    None => Ok(Expr::undefined()),\n",
    tests: ["test/iterator-forms.test.ts", "-t", "generic_flatten_of_vecs"],
  },
  {
    name: "rc-eq-fact-always-true",
    breaks: "a caller gives `TEq` `true` of a type only `PartialEq`, so `==` of `Rc`s of `f64::NAN` in generic code is `true` of one allocation, where std calls `eq` (#931)",
    file: "src/lower/std_types/number/generic.rs",
    find: "            return Ok(Expr::bool(holds));\n        }\n        // Of a type whose fact is the facts of what it holds",
    replace: "            return Ok(Expr::bool(true));\n        }\n        // Of a type whose fact is the facts of what it holds",
    tests: ["test/corpus.test.ts", "-t", "^rc_eq_generic_by_address_first.rs$"],
  },
  {
    name: "rc-eq-fact-unrefused",
    breaks: "a caller gives `TEq` of a fieldless enum with its own `eq`, whose `===` two allocations share, so a generic `==` of `Rc`s of it skips its `eq` (#931)",
    file: "src/lower/std_types/number/generic.rs",
    find: "                self.check_rc_allocations(&format!(\"std::rc::Rc<{of}>\"), of, span)?;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^rc_eq_generic_of_a_fieldless_enum_refused.rs$"],
  },
];
