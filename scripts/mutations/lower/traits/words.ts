// Mutations of src/lower/traits/words.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "reference-impl-name",
    breaks:
      "an impl for `&'a str` is exported as rustc prints its type, `__a_strNamed`, not `refStrNamed`",
    file: "src/lower/traits/words.rs",
    find: "        ty::Ref(_, inner, mutability) => {\n            let inner = js_word(&type_word(tcx, *inner));\n",
    replace:
      "        ty::Ref(_, inner, mutability) if false => {\n            let inner = js_word(&type_word(tcx, *inner));\n",
    tests: ["test/traits.test.ts", "-t", "dictionaries are explicit"],
  },
  {
    name: "impl-names-collide",
    breaks:
      "`Vec<i32>`'s and `Vec<String>`'s impls of one trait have one name, which is an error",
    file: "src/lower/traits/words.rs",
    find: "    if alike.len() < 2 {\n        return name;\n    }",
    replace: "    if alike.len() < 200 {\n        return name;\n    }",
    tests: ["test/corpus.test.ts", "-t", "impl_names"],
  },
  {
    name: "impl-trait-args-peeled",
    breaks:
      "`Convert<u8>`'s and `Convert<&u8>`'s impls are named alike, without the reference",
    file: "src/lower/traits/words.rs",
    find: "                (Some(ty), _) => js_word(&full_type_word(tcx, ty)),",
    replace:
      "                (Some(ty), _) => js_word(&full_type_word(tcx, ty.peel_refs())),",
    tests: ["test/corpus.test.ts", "-t", "impl_names"],
    snapshots: true,
  },
  {
    name: "impl-locals-unnumbered",
    breaks:
      "two types of one name, local to two functions, have impls of one name",
    file: "src/lower/traits/words.rs",
    find: '        Some(at) if at > 0 => format!("{full}{}", at + 1),',
    replace: "        Some(at) if at > 0 => full,",
    tests: ["test/corpus.test.ts", "-t", "impl_names"],
  },
  {
    name: "impl-name-consts-unnamed",
    breaks:
      "`Scaled<10>`'s and `Scaled<100>`'s impls are numbered, not named by their values: right, but not the JS a person writes",
    file: "src/lower/traits/words.rs",
    find: "                (None, Some(c)) => js_word(&c.to_string()),",
    replace: "                (None, Some(_)) => continue,",
    tests: ["test/corpus.test.ts", "-t", "const_generic_traits"],
    snapshots: true,
  },
  {
    name: "same-named-traits-collide",
    breaks:
      "two traits of one name for one type, serde's `de::Error` and `ser::Error`, aren't told apart by their modules' words, `de_Error` and `ser_Error`, but spelled in full",
    file: "src/lower/traits/words.rs",
    find: "            .any(|&other| tcx.impl_trait_id(other) != tr.def_id && short_of(other) == short);",
    replace:
      "            .any(|&other| tcx.impl_trait_id(other) != tr.def_id && short_of(other) == short && false);",
    tests: ["test/corpus.test.ts", "-t", "impl_names"],
    snapshots: true,
  },
  {
    name: "impl-trait-drop-named-as-written",
    breaks:
      "the drop of an `impl fmt::Display` argument is named `dropimpl fmt::Display`, which isn't JS",
    file: "src/lower/traits/words.rs",
    find: '    let Some(bound) = name.as_str().strip_prefix("impl ") else {',
    replace:
      '    let Some(bound) = name.as_str().strip_prefix("impl ").filter(|_| false) else {',
    tests: ["test/corpus.test.ts", "-t", "drop_generic"],
  },
  {
    name: "evidence-word-of-array-mangled",
    breaks:
      "a bound of an array's dictionary is named by the type as rustc prints it, `_T__N_Default`, not `TArrayDefault`",
    file: "src/lower/traits/words.rs",
    find: '        ty::Array(item, _) => format!("{}Array", evidence_word(tcx, *item)),\n',
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^array_defaults.rs$"],
    snapshots: true,
  },
  {
    name: "evidence-word-of-struct-path",
    breaks:
      "a bound of a struct's dictionary is named by its path, `std__num__Wrapping_T_Mul`, not `WrappingMul`",
    file: "src/lower/traits/words.rs",
    find: "        ty::Adt(..) => type_word(tcx, ty),\n",
    replace: "",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(array_defaults|trait_default_bounds).rs$",
    ],
    snapshots: true,
  },
  {
    name: "ref-evidence-unnamed",
    breaks:
      "a bound on `&'a mut C`'s dictionary is named `_mut_CIntoIterator`, not `CRefMutIntoIterator`",
    file: "src/lower/traits/words.rs",
    find: '            format!("{}{kind}", evidence_word(tcx, *inner))\n',
    replace: "            ty.to_string()\n",
    tests: ["test/corpus.test.ts", "-t", "^into_iter_of_ref_param.rs$"],
    snapshots: true,
  },
  {
    name: "derive-parameter-underscored",
    breaks:
      "a derive's own type parameter's evidence is named `__HHasher`, not `HHasher`",
    file: "src/lower/traits/words.rs",
    find: "        let unmarked = name.as_str().trim_start_matches('_');\n",
    replace: "        let unmarked = name.as_str();\n",
    tests: ["test/corpus.test.ts", "-t", "^hash_values.rs$"],
    snapshots: true,
  },
  {
    name: "impls-of-alike-traits-unnumbered",
    breaks: "two traits of one name in modules of one name, each a `const _`'s block's `Subtrait`, aren't numbered apart, and zerocopy's library is refused: \"generated trait implementation name collides\" (#1298)",
    file: "src/lower/traits/words.rs",
    find: "        .filter(|&other| short_of(other) == short && named(other, false, &module_of(other)) == name)\n",
    replace: "        .filter(|&other| tcx.impl_trait_id(other) == tcx.impl_trait_id(id) && named(other, false, &module_of(other)) == name)\n",
    tests: ["test/corpus.test.ts", "-t", "^traits_of_one_name_in_const_blocks.rs$"],
  },
  {
    name: "impl-trait-word-uncapitalized",
    breaks: "what a trait method's `impl Trait` is called in its dictionary's keys is its method's name as it is, `$drophidden`, not as an associated type's reads, `$dropHidden` (#1296)",
    file: "src/lower/traits/words.rs",
    find: "                .map_or(name.clone(), |first| first.to_uppercase().chain(chars).collect())\n",
    replace: "                .map_or(name.clone(), |first| std::iter::once(first).chain(chars).collect())\n",
    tests: ["test/crates.test.ts", "-t", "drops a consumer's impl Trait by its dictionary"],
  },
  {
    name: "impl-trait-of-any-generic-carried",
    breaks: "a trait method's `impl Trait` of a type parameter not bound by `Copy` is taken to have one drop of every instance, its dictionary's, made of `()`, which drops nothing of what an instance holds (#1575)",
    file: "src/lower/traits/words.rs",
    find: "                    if Some(tr.def_id()) == copy && tr.self_ty().skip_binder() == param_ty)\n",
    replace: "                    if true || Some(tr.def_id()) == copy && tr.self_ty().skip_binder() == param_ty)\n",
    tests: ["test/diagnostics.test.ts", "-t", "an impl Trait of a method's own generic held"],
  },
  {
    name: "impl-trait-of-own-generics-uncarried",
    breaks: "a trait method's `impl Trait` of generics of its own is never carried, so generic code holding `S::counted::<5>(4)` beside a destructor is refused (#1575)",
    file: "src/lower/traits/words.rs",
    find: "    generics.own_params.iter().all(|param| match param.kind {\n",
    replace: "    generics.own_params.is_empty() && generics.own_params.iter().all(|param| match param.kind {\n",
    tests: ["test/corpus.test.ts", "-t", "^impl_trait_of_a_method_with_its_own_generics_dropped.rs$"],
  },
];
