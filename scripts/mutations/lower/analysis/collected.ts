// Mutations of src/lower/analysis/collected.rs, the bodies the crate lowers, collected before anything is analyzed.
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "type-constants-of-a-private-type",
    breaks: "a private type's constants make an object nothing can name (#498)",
    file: "src/lower/analysis/collected.rs",
    find: "            .is_some_and(|local| tcx.effective_visibilities(()).is_exported(local))\n",
    replace: "            .is_some()\n",
    tests: ["test/impl-constants.test.ts"],
  },
  {
    name: "type-constants-of-a-class",
    breaks: "a class's constant, its prototype's property, is taken for a plain type's object's (#498)",
    file: "src/lower/analysis/collected.rs",
    find: "        && crate::lower::classes::member(tcx, def_id).is_none()\n",
    replace: "",
    tests: ["test/impl-constants.test.ts"],
  },
  {
    name: "type-constants-of-a-generic-impl",
    breaks: "a generic impl's constant, which rustc can't say for every type at once, is taken for its object's (#498)",
    file: "src/lower/analysis/collected.rs",
    find: "        && tcx.generics_of(def_id).count() == 0\n",
    replace: "",
    tests: ["test/impl-constants.test.ts"],
  },
  {
    name: "user-hash-lowered",
    breaks:
      "a type's own `Hash`'s `hash`, and its `Future`'s `poll`, are lowered, and what they call refused, though nothing calls them",
    file: "src/lower/analysis/collected.rs",
    find: "                    && !is_unlowered_impl(tcx, parent)\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^(user_hash|unlowered_impls).rs$"],
  },
  {
    name: "js-object-deref-lowered",
    breaks:
      "a JS class's `Deref` to the class it extends is lowered, a function making a pointer to nothing",
    file: "src/lower/analysis/collected.rs",
    find: "                    && !is_js_object_deref(tcx, parent)\n",
    replace: "",
    tests: [
      "test/js-class-deref.test.ts",
      "-t",
      "a JS class's Deref to the class it extends is no JS",
    ],
  },
  {
    name: "coded-inline-consts-every",
    breaks:
      "every `const { .. }` block is a module constant, one rustc's value says too",
    file: "src/lower/analysis/collected.rs",
    find: "                .is_none()\n            {\n                coded.push(local);",
    replace:
      "                .is_none_or(|_| true)\n            {\n                coded.push(local);",
    tests: ["test/corpus.test.ts", "-t", "const_blocks_and_let_guards"],
    snapshots: true,
  },
  {
    name: "trait-default-const-unread",
    breaks:
      "a trait's default constant's initializer isn't read, so a library's dictionary can't copy it in",
    file: "src/lower/analysis/collected.rs",
    find: "\n                    || tcx.trait_of_assoc(def_id.to_def_id()).is_some())",
    replace: ")",
    tests: [
      "test/lowering-structs-enums.test.ts",
      "-t",
      "default constant of Self",
    ],
  },
  {
    name: "mir-evaluated-body-lost",
    breaks: "a \`const fn\` a pattern's constant evaluates has no MIR, refused",
    file: "src/lower/analysis/collected.rs",
    find: "        body.mir = match (built.is_stolen() || promoted.is_stolen(), evaluated(local)) {\n",
    replace:
      "        body.mir = match (built.is_stolen() || promoted.is_stolen(), false) {\n",
    tests: ["test/corpus.test.ts", "-t", "const_fn_in_patterns"],
  },
  {
    name: "impl-const-any-type",
    breaks:
      "an impl's constant of a number is lowered as code, `S.ZERO`, not its value, `0`",
    file: "src/lower/analysis/collected.rs",
    find: "    coded\n        && (tcx.trait_impl_of_assoc(def_id).is_none()\n",
    replace:
      "    true\n        && (tcx.trait_impl_of_assoc(def_id).is_none()\n",
    tests: ["test/corpus.test.ts", "-t", "^impl_fn_consts.rs$"],
    snapshots: true,
  },
  {
    name: "impl-const-generic-trait-impl",
    breaks:
      "a generic trait impl's constant, of its parameters' evidence, is lowered once as a constant of its module, which has none",
    file: "src/lower/analysis/collected.rs",
    find: "    coded\n        && (tcx.trait_impl_of_assoc(def_id).is_none()\n",
    replace: "    coded\n        && (true\n",
    tests: ["test/corpus.test.ts", "-t", "^impl_fn_consts.rs$"],
  },
  {
    name: "uninit-const-block-coded",
    breaks:
      "a `const { MaybeUninit::uninit() }` block is a constant of its module, made as code, not `undefined` where it's used",
    file: "src/lower/analysis/collected.rs",
    find: "                && crate::lower::representation::const_value_js(\n                    tcx,\n                    &IndexSet::default(),\n",
    replace: "                && crate::lower::eval_const(\n                    tcx,\n",
    tests: ["test/corpus.test.ts", "-t", "^const_block_uninit_array.rs$"],
    snapshots: true,
  },
  {
    name: "unlowered-helpers-lowered",
    breaks:
      "a private function only a type's own `io::Read` calls is lowered, JS no one calls (ADR 0425)",
    file: "src/lower/analysis/collected.rs",
    find: "        if found.is_empty() {\n            return unreached;\n",
    replace: "        if true {\n            return unreached;\n",
    tests: ["test/corpus.test.ts", "-t", "^unlowered_helpers.rs$"],
    snapshots: true,
  },
  {
    name: "derived-hash-unlowered",
    breaks: "a derived `Hash`'s method isn't lowered, so hashing it is refused",
    file: "src/lower/analysis/collected.rs",
    find: "        && [StdItem::Debug, StdItem::Hash].into_iter().any(|item| {\n",
    replace: "        && [StdItem::Debug].into_iter().any(|item| {\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_values.rs$"],
  },
  {
    name: "impl-const-body-dropped",
    breaks:
      "a trait impl's constant keeps no initializer, so a generic impl's, `Wrapping(T::ZERO)`, or one rustc can't say, `String::new()`, is refused",
    file: "src/lower/analysis/collected.rs",
    find: "                (tcx.trait_impl_of_assoc(def_id.to_def_id()).is_some()\n",
    replace: "                (false\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(impl_const_string|generic_impl_consts)\\.rs$",
    ],
  },
  {
    name: "impl-const-without-body",
    breaks:
      "an impl's `type const`, which has no body, is lowered, and rustc crashes asking for one",
    file: "src/lower/analysis/collected.rs",
    find: "\n                    && tcx.hir_maybe_body_owned_by(def_id).is_some()\n",
    replace: "\n",
    tests: ["test/diagnostics.test.ts", "-t", "a type constant"],
  },
  {
    name: "class-trait-field-calls-unread",
    breaks:
      "a call of a trait of classes' method that gives a field, outside its defaults, calls a method that isn't, `this.items()`, not the field (rd304)",
    file: "src/lower/analysis/collected.rs",
    find: "            mir.read_fields(tcx, typing_env);\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^class_trait_field_borrows.rs$"],
  },
  {
    name: "type-constant-initializer-dropped",
    breaks: "a pub type's constant no value tree says, semver's `VersionReq::STAR` of a `Vec`, keeps no initializer, so it's refused (#513)",
    file: "src/lower/analysis/collected.rs",
    find: "                impl_const(tcx, def_id.to_def_id()) || exported_value_const(tcx, def_id.to_def_id())\n",
    replace: "                impl_const(tcx, def_id.to_def_id())\n",
    tests: ["test/impl-constants.test.ts", "-t", "no value tree"],
  },
];
