// Mutations of src/lower/drops/glue.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "glue-parts-unchained",
    breaks:
      "a struct's, a tuple's or a variant's parts after one whose destructor panics are never dropped, its throw past them (#416)",
    file: "src/lower/drops/glue.rs",
    find: "            if cleanup || rest.is_empty() || !may_throw(&part) {\n",
    replace: "            if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_parts_past_a_panic.rs$"],
  },
  {
    name: "glue-rest-each-in-its-try",
    breaks:
      "drop glue's `catch` of a part runs each later part in a `try` of its own, so where one of those panics too the parts after it still drop before the abort, `drop 4`, where Rust aborts at once (#865)",
    file: "src/lower/drops/glue.rs",
    find: "                UnwindAction::Cleanup(cleanup) => self.glue_from(glue, cleanup, true, made, out)?,\n",
    replace: "                UnwindAction::Cleanup(_) => rest.clone(),\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_glue_second_panic_aborts.rs$"],
  },
  {
    name: "glue-cleanup-ladders",
    breaks:
      "drop glue's cleanup puts each part that may throw in a `try` whose `catch` only throws again, its cleanup's `unwind terminate`: JS no one writes (form only, #893)",
    file: "src/lower/drops/glue.rs",
    find: "            if cleanup || rest.is_empty() || !may_throw(&part) {\n",
    replace: "            if rest.is_empty() || !may_throw(&part) {\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_glue_of_an_enum_with_its_own_drop.rs$"],
    snapshots: true,
  },
  {
    name: "glue-variant-otherwise-first",
    breaks:
      "an enum's variant the shim's switch lists by its discriminant is dropped as the `otherwise` one, its fields read of another variant",
    file: "src/lower/drops/glue.rs",
    find: "                .find(|&(value, _)| value == discriminant.val)\n",
    replace: "                .find(|&(value, _)| value == discriminant.val && false)\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_glue_of_an_enum_with_its_own_drop.rs$"],
  },
  {
    name: "boxed-some-dropped-boxed",
    breaks: "a generic `Option<T>`'s boxed `Some` is given to `T`'s drop as the box",
    file: "src/lower/drops/glue.rs",
    find: "                            true => self.some_value(value.clone()),",
    replace: "                            true => value.clone(),",
    tests: ["test/corpus.test.ts", "-t", "option_map_drops"],
  },
  {
    name: "untagged-drop-part",
    breaks: "dropping an untagged enum reads a `_0` its value hasn't",
    file: "src/lower/drops/glue.rs",
    find: "                        let part = match self.untagged(ty).is_some() {\n",
    replace: "                        let part = match false {\n",
    tests: ["test/corpus.test.ts", "-t", "untagged_enums"],
  },
  {
    name: "dyn-tail-dropped-as-pair",
    breaks:
      "a struct's `dyn` last field is dropped as a pair, `fat.inner.impl.$drop`, not through the struct's `impl`, and throws",
    file: "src/lower/drops/glue.rs",
    find: "                    ty::Dynamic(..) => {\n                        let dictionary = Expr::member(value, js::DICTIONARY);\n",
    replace:
      "                    ty::Dynamic(..) if false => {\n                        let dictionary = Expr::member(value, js::DICTIONARY);\n",
    tests: ["test/corpus.test.ts", "-t", "^unsized_struct_dyn.rs$"],
  },
  {
    name: "glue-own-drop-unrun",
    breaks: "a type's own `drop` is left out of its drop glue, only its fields dropped",
    file: "src/lower/drops/glue.rs",
    find: "                if let Some(call) = self.own_drop(value, ty, span, out)? {\n",
    replace: "                if let Some(call) = self.own_drop(value, ty, span, out)?.filter(|_| false) {\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_parts_past_a_panic.rs$"],
  },
  {
    name: "enum-variants-all-dropped",
    breaks: "an enum's drop runs every variant's fields' drops, not the one it holds, `if (true)` of each",
    file: "src/lower/drops/glue.rs",
    find: "            } else if adt.variants().len() == 1 {\n",
    replace: "            } else if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_scopes.rs$"],
  },
  {
    name: "option-drop-tested-as-variant",
    breaks: "an `Option`'s drop tests its `Some` with the variant test a match writes, `!== undefined`, not `value != null` (form only)",
    file: "src/lower/drops/glue.rs",
    find: "            let test = if self.option_of(glue.ty).is_some() {\n",
    replace: "            let test = if false {\n",
    tests: ["test/corpus.test.ts", "-t", "^drop_scopes.rs$"],
    snapshots: true,
  },
];
