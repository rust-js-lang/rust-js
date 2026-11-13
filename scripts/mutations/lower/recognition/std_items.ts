// Mutations of src/lower/recognition/std_items.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "passed-on-of-any-self",
    breaks:
      "a method of `self` std's impl for a `&mut I` makes of another is taken as `I`'s, so a `&mut` to an iterator of the crate's own folds by its own `fold`, where std's impl folds by `I`'s `try_fold`, its `next` (#1309)",
    file: "src/lower/recognition/std_items.rs",
    find: "        if !matches!(*sig.inputs().first()?.kind(), ty::Ref(_, of, Mutability::Mut) if of == tcx.types.self_param) {",
    replace: "        if false {",
    tests: ["test/corpus.test.ts", "-t", "^std_method_of_a_mut_to_a_held_mut.rs$"],
  },
  {
    name: "passed-on-a-default",
    breaks:
      "a method std's impl for a `&mut I` leaves its trait's default, `by_ref()`, which runs of the `&mut I` itself, is taken as `I`'s (#1309)",
    file: "src/lower/recognition/std_items.rs",
    find: "            self.tcx.impl_of_assoc(instance.def_id()).is_some() && self.is_std(instance.def_id())",
    replace: "            self.is_std(instance.def_id())",
    tests: ["test/corpus.test.ts", "-t", "^(std_method_of_a_mut_to_a_held_mut|by_ref_lends|boxed_iterator_stepped_as_what_it_boxes).rs$"],
  },
  {
    name: "passed-on-of-shared-self",
    breaks:
      "a method of `&self` std's impl for a `&mut I` passes on is `I`'s, given `*self` of a `&&mut I`, so `by_ref().len()` of a `Vec`'s iterator reads `NaN` (#1309)",
    file: "src/lower/recognition/std_items.rs",
    find: "        if !matches!(*sig.inputs().first()?.kind(), ty::Ref(_, of, Mutability::Mut) if of == tcx.types.self_param) {",
    replace: "        if !matches!(*sig.inputs().first()?.kind(), ty::Ref(_, of, _) if of == tcx.types.self_param) {",
    tests: ["test/corpus.test.ts", "-t", "^by_ref_lends.rs$"],
  },
  {
    name: "zip-by-index-pure",
    breaks:
      "a zip by index ignores whether its first side runs code, so a slice's plain zip, which no one can tell apart, is `$trustedZip` too, noisier (ADR 0973)",
    file: "src/lower/recognition/std_items.rs",
    find: "    is_random_access(a) && is_random_access(b) && steps_code(tcx, typing_env, a)\n",
    replace: "    is_random_access(a) && is_random_access(b)\n",
    tests: ["test/corpus.test.ts", "-t", "^(zip_by_index|lazy_chains).rs$"],
    snapshots: true,
  },
  {
    name: "std-questions-impure",
    breaks:
      "`len()` and the like are taken as calls that may panic, so `queue.borrow().len()` of a guard dropped after it is `const { length: result } = $borrow(queue); return result;`, not `return $borrow(queue).length;` (ADR 0301; drop_temporaries' `make(\"c\").name().is_empty()` no longer shows it, #606)",
    file: "src/lower/recognition/std_items.rs",
    find: '"is_err" if tcx.trait_of_assoc(id).is_none() => {',
    replace: '"is_err" if false => {',
    tests: ["test/corpus.test.ts", "-t", "^refcell_vec_bounds.rs$"],
    snapshots: true,
  },
  {
    name: "unchecked-may-panic",
    breaks:
      "`unwrap_unchecked()` is taken as one that may panic, so a consumer that gives the library's `first` a value with a destructor compiles its own, to drop it where it panics",
    file: "src/lower/recognition/std_items.rs",
    find: '        "unwrap_unchecked" => Some(PureStd::Unchecked),\n',
    replace: "",
    tests: ["test/bindings.test.ts", "-t", "can't throw"],
  },
  {
    name: "once-lock-unknown",
    breaks: "a `OnceLock` isn't known, its methods refused",
    file: "src/lower/recognition/std_items.rs",
    find: '                StdItem::OnceCell => ("OnceCell", "OnceLock"),\n',
    replace: '                StdItem::OnceCell => ("OnceCell", ""),\n',
    tests: ["test/corpus.test.ts", "-t", "once_cells"],
  },
  {
    name: "lazy-lock-unknown",
    breaks: "a `LazyLock` isn't known, a `static` one refused",
    file: "src/lower/recognition/std_items.rs",
    find: '                _ => ("LazyCell", "LazyLock"),\n',
    replace: '                _ => ("LazyCell", ""),\n',
    tests: ["test/corpus.test.ts", "-t", "lazy_cells"],
  },
  {
    name: "std-alone-crate-types",
    breaks: "a bound of the crate's type is taken as std's",
    file: "src/lower/recognition/std_items.rs",
    find: "                ty::Adt(adt, _) => self.is_std(adt.did()) && !self.is_borrowed_cell(t),",
    replace: "                ty::Adt(..) => true,",
    tests: ["test/corpus.test.ts", "-t", "refcell_held_while_asked"],
  },
  {
    name: "maybe-uninit-by-diagnostic",
    breaks: "`MaybeUninit` is never recognized, its values errors",
    file: "src/lower/recognition/std_items.rs",
    find: "        StdItem::MaybeUninit => tcx.is_lang_item(id, LangItem::MaybeUninit),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "box_uninit"],
  },
  {
    name: "occupied-entry-unknown",
    breaks: "an `OccupiedEntry` is refused",
    file: "src/lower/recognition/std_items.rs",
    find: '        matches!(ty.kind(), ty::Adt(adt, _) if map_entry_item(self.tcx, adt.did(), "OccupiedEntry"))',
    replace:
      '        matches!(ty.kind(), ty::Adt(adt, _) if map_entry_item(self.tcx, adt.did(), "NotOccupiedEntry"))',
    tests: ["test/corpus.test.ts", "-t", "btree_end_entries"],
  },
  {
    name: "env-args-unknown",
    breaks: "`env::args()` is refused, not the process's arguments",
    file: "src/lower/recognition/std_items.rs",
    find: '    ("std::env::args", NodeOnly::Process(ProcessOp::Args)),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^env_args.rs$"],
  },
  {
    name: "env-var-unknown",
    breaks: "`env::var(key)` is refused, not the process's environment",
    file: "src/lower/recognition/std_items.rs",
    find: '    ("std::env::var", NodeOnly::Process(ProcessOp::Var)),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^env_vars.rs$"],
  },
  {
    name: "hashbrown-maps-unknown",
    breaks:
      "hashbrown's `HashMap` and `HashSet` aren't std's, and rust-js walks into its table",
    file: "src/lower/recognition/std_items.rs",
    find: "if let Some(bound) = bound_map_item(tcx, id) => {",
    replace:
      "if let Some(bound) = bound_map_item(tcx, id).filter(|_| false) => {",
    tests: ["test/bound-maps.test.ts", "-t", "hashbrown"],
  },
  {
    name: "hashbrown-entries-unknown",
    breaks: "hashbrown's map's `VacantEntry` isn't std's `[m, key]`",
    file: "src/lower/recognition/std_items.rs",
    find: "        Some(item) => item == name,\n",
    replace: "        Some(_) => false,\n",
    tests: ["test/bound-maps.test.ts", "-t", "hashbrown"],
  },
  {
    name: "var-os-unknown",
    breaks: "`env::var_os(key)` is an error, where it's the variable or `None`",
    file: "src/lower/recognition/std_items.rs",
    find: '    ("std::env::var_os", NodeOnly::Process(ProcessOp::VarOs)),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^env_vars.rs$"],
  },
  {
    name: "vars-os-unknown",
    breaks: "`env::vars_os()` is an error, where it's `vars()`'s pairs",
    file: "src/lower/recognition/std_items.rs",
    find: '    ("std::env::vars_os", NodeOnly::Process(ProcessOp::VarsOs)),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^env_vars.rs$"],
  },
  {
    name: "c-variadic-unseen",
    breaks:
      "a C-variadic function's last parameter is one value, not what it's given past the rest",
    file: "src/lower/recognition/std_items.rs",
    find: "        && tcx.fn_sig(def_id).skip_binder().skip_binder().c_variadic()\n",
    replace: "        && false\n",
    tests: ["test/corpus.test.ts", "-t", "^c_variadic.rs$"],
  },
  {
    name: "fmt-write-function-unlowered",
    breaks: "`fmt::write(w, args)` is refused, where it's `w.write_fmt(args)`",
    file: "src/lower/recognition/std_items.rs",
    find: '            && std_path(tcx, def_id).ends_with("fmt::write")\n',
    replace: "            && false\n",
    tests: ["test/corpus.test.ts", "-t", "^fmt_write_function.rs$"],
  },
  {
    name: "memchr-refused-recognized",
    breaks:
      "a call into a memchr Cargo refused, not qualified, is lowered as the qualified one's",
    file: "src/lower/recognition/std_items.rs",
    find: "            && crate::lower::refusal::uncompiled(self.tcx, id).is_none()\n",
    replace: "",
    tests: [
      "test/registry-crates.test.ts",
      "-t",
      "a memchr that isn't crates.io's qualified one",
    ],
  },
  {
    name: "memrchr-forward",
    breaks: "`memrchr` finds the first, not the last",
    file: "src/lower/recognition/std_items.rs",
    find: '            "memrchr" | "memrchr2" | "memrchr3" => MemchrOp::Find { back: true },\n',
    replace:
      '            "memrchr" | "memrchr2" | "memrchr3" => MemchrOp::Find { back: false },\n',
    tests: ["test/shipped-crates.test.ts", "-t", "memchr's searches run"],
  },
  {
    name: "concat-macro-unknown",
    breaks:
      "what std's `concat!` made isn't known as its, and so is one string, not its pieces",
    file: "src/lower/recognition/std_items.rs",
    find: 'is_some_and(|id| tcx.is_diagnostic_item(Symbol::intern("macro_concat"), id))',
    replace:
      'is_some_and(|id| !tcx.is_diagnostic_item(Symbol::intern("macro_concat"), id))',
    tests: [
      "test/lowering-strings.test.ts",
      "-t",
      "a concat! of literals is its pieces joined by",
    ],
  },
  {
    name: "noinline-of-any-inline",
    breaks: "`#[inline]` and `#[inline(always)]` are `@noinline` too",
    file: "src/lower/recognition/std_items.rs",
    find: "Inline(rustc_hir::attrs::InlineAttr::Never, _))\n",
    replace: "Inline(_, _))\n",
    tests: ["test/emission.test.ts", "-t", "inline\\(never\\)"],
  },
  {
    name: "ne-override-trusted",
    breaks:
      "a `PartialEq` whose `ne` is its own is taken as equal only to itself, as its `eq` says (ADR 0864)",
    file: "src/lower/recognition/std_items.rs",
    find: '            if trait_id == partial_eq && implementors.contains_key(&trait_method(tcx, partial_eq, "ne")) {\n',
    replace:
      '            if false && implementors.contains_key(&trait_method(tcx, partial_eq, "ne")) {\n',
    tests: [
      "test/lowering-collections.test.ts",
      "-t",
      "a type equal only to itself",
    ],
  },
  {
    name: "bound-eq-own-ne-trusted",
    breaks:
      "a binding crate's `PartialEq` whose `eq` is bound to `===` and whose `ne` is its own, which no binding names, is `===` and `!==`, its `ne` never asked (ADR 1016)",
    file: "src/lower/recognition/std_items.rs",
    find: "                && !implementors.contains_key(&ne)\n",
    replace: "",
    tests: ["test/webapi-bindings.test.ts", "-t", "equal only to itself"],
  },
  {
    name: "bound-eq-form-unchecked",
    breaks:
      "a binding crate's `PartialEq` whose `eq` is any binding is `===`, whatever JS it names (ADR 1016)",
    file: "src/lower/recognition/std_items.rs",
    find: "                    super::super::bindings::is_binding(tcx, eq)\n                        && matches!(",
    replace:
      "                    super::super::bindings::is_binding(tcx, eq)\n                        || matches!(",
    tests: ["test/webapi-bindings.test.ts", "-t", "equal only to itself"],
  },
  {
    name: "libtest-by-name",
    breaks:
      "a crate named `test` that rust-js didn't compile is libtest, so its `black_box` is libtest's, its argument, where a call of it is refused (#455, ADR 1054)",
    file: "src/lower/recognition/std_items.rs",
    find: '    tcx.crate_name(id.krate).as_str() == "test" && from_sysroot(tcx, id)\n',
    replace: '    tcx.crate_name(id.krate).as_str() == "test"\n',
    tests: ["test/crates.test.ts", "-t", "of no JS named test or std"],
  },
  {
    name: "std-type-ids-hashed-beside-their-name",
    breaks:
      "std's `TypeId`s have their hash beside a program's crate named `core`, and not in a crate that doesn't see it, so its `Option<i32>` isn't the one another crate boxed (#455, ADR 1054)",
    file: "src/lower/recognition/std_items.rs",
    find: "    !is_std_crate(tcx, id)\n        && (tcx.crates(()).iter().chain([&rustc_span::def_id::LOCAL_CRATE]))\n",
    replace: "    (tcx.crates(()).iter().chain([&rustc_span::def_id::LOCAL_CRATE]))\n",
    tests: ["test/crates.test.ts", "-t", "type_ids_beside_a_crate_named_core"],
  },
  {
    name: "type-id-census-without-the-crate-compiled",
    breaks:
      "a crate named `core`, compiled, isn't counted among the crates of its name, so its own `option::Option<i32>` has std's `Option<i32>`'s `TypeId` (#455, ADR 1054)",
    file: "src/lower/recognition/std_items.rs",
    find: "        && (tcx.crates(()).iter().chain([&rustc_span::def_id::LOCAL_CRATE]))\n",
    replace: "        && (tcx.crates(()).iter())\n",
    tests: ["test/crates.test.ts", "-t", "crate_named_core"],
  },
  {
    name: "std-memchr-unanswered",
    breaks: "core's own `memchr`, which `read_until` calls, is no recognizer's, and std's body is refused",
    file: "src/lower/recognition/std_items.rs",
    find: "                \"std::slice::memchr::memchr\" => Some(MemchrOp::Find { back: false }),\n",
    replace: "",
    tests: ["test/std-package.test.ts", "-t", "the committed @rust-js/std is std"],
  },
  {
    name: "box-iterator-not-passed-through",
    breaks: "a std iterator method of a `Box<I>` is the box's, not `I`'s: `next()` of a `Box<Map<..>>` steps the `&mut` to the box, `it.next is not a function` (#1278)",
    file: "src/lower/recognition/std_items.rs",
    find: "        (iterator_trait\n            && inner.is_sized(tcx, self.typing_env)\n",
    replace: "        (false\n            && inner.is_sized(tcx, self.typing_env)\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_iterator_stepped_as_what_it_boxes.rs$"],
  },
  {
    name: "box-iterator-defaults-not-passed-through",
    breaks: "a default of a `Box<I>`'s, `take(n)`, `map(f)` or `sum()`, is the box's, not `I`'s, though the box's value is `I`'s: `boxed.take is not a function` (#1278)",
    file: "src/lower/recognition/std_items.rs",
    find: "            && (!(lends\n                || self.is_user_iterator(inner)\n                || crate::lower::generic_facts::boxed_steps_given(tcx, inner, self.typing_env))\n                || self.std_impl_writes(def_id, generic_args)))\n",
    replace: "            && self.std_impl_writes(def_id, generic_args))\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_iterator_stepped_as_what_it_boxes.rs$"],
  },
  {
    name: "box-iterator-of-a-mut-ref-not-passed-through",
    breaks: "`(&mut boxed).take(n)` of a `Box<I>` takes the `&mut` to the box, not to `I`, which it holds (#1278)",
    file: "src/lower/recognition/std_items.rs",
    find: "            ty::Ref(region, boxed, Mutability::Mut) if by_value => {\n",
    replace: "            ty::Ref(region, boxed, Mutability::Mut) if false => {\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_iterator_stepped_as_what_it_boxes.rs$"],
  },
  {
    name: "path-canonicalize-unrecognized",
    breaks: "`p.canonicalize()`, a path's method, is no call of std's files, and is refused (#1420)",
    file: "src/lower/recognition/std_items.rs",
    find: "    (\"std::path::Path::canonicalize\", NodeOnly::Fs(FsOp::PathCanonicalize)),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^fs_canonicalize_and_current_dir.rs$"],
  },
  {
    name: "box-user-iterator-default-passed-through",
    breaks: "a default of a `Box` of the crate's iterator, `count()` or `fold()`, is passed on to the iterator's own, where std's `Box<I>` steps it by `next` (#1601)",
    file: "src/lower/recognition/std_items.rs",
    find: "            && (!(lends\n                || self.is_user_iterator(inner)\n                || crate::lower::generic_facts::boxed_steps_given(tcx, inner, self.typing_env))\n                || self.std_impl_writes(def_id, generic_args)))\n",
    replace: "            && (!(lends\n                || crate::lower::generic_facts::boxed_steps_given(tcx, inner, self.typing_env))\n                || self.std_impl_writes(def_id, generic_args)))\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_iterator_steps_as_std_box_does.rs$"],
  },
  {
    name: "box-param-default-passed-through",
    breaks: "a default of generic code's `Box<I>`, `count()`, is passed on to `I`'s, its caller's dictionary's own `count`, which std's `Box<I>` never calls (#1611)",
    file: "src/lower/recognition/std_items.rs",
    find: "            && (!(lends\n                || self.is_user_iterator(inner)\n                || crate::lower::generic_facts::boxed_steps_given(tcx, inner, self.typing_env))\n                || self.std_impl_writes(def_id, generic_args)))\n",
    replace: "            && (!(lends || self.is_user_iterator(inner)) || self.std_impl_writes(def_id, generic_args)))\n",
    tests: ["test/corpus.test.ts", "-t", "^boxed_iterator_steps_as_std_box_does.rs$"],
  },
  {
    name: "non-null-by-diagnostic-item",
    breaks: "`NonNull`, a lang item since 1.100, is asked of its old diagnostic item, so it's no pointer and its pattern-typed field is refused (ADR 0109, amended)",
    file: "src/lower/recognition/std_items.rs",
    find: "        StdItem::NonNull => tcx.is_lang_item(id, LangItem::NonNull),\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^non_null.rs$"],
  },
];
