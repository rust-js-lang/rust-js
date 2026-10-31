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
      "two traits of one name for one type, serde's `de::Error` and `ser::Error`, are named the same, an error",
    file: "src/lower/traits/words.rs",
    find: '        .any(|&other| tcx.impl_trait_id(other) != tr.def_id && named(other, false, "") == short);',
    replace:
      '        .any(|&other| tcx.impl_trait_id(other) != tr.def_id && named(other, false, "") == short && false);',
    tests: ["test/corpus.test.ts", "-t", "impl_names"],
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
];
