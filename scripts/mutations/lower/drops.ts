// Mutations of src/lower/drops.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "drop-function-in-branch",
    breaks:
      "a drop function of a trait's default method is declared where it's first needed, a branch another call isn't in",
    file: "src/lower/drops.rs",
    find: "                Some(hoisted) => hoisted.defs.push(def),\n",
    replace: "                Some(_) => out.push(def),\n",
    tests: ["test/corpus.test.ts", "-t", "default_method_drops"],
  },
  {
    name: "closure-drop-skipped",
    breaks: "a closure dropped uncalled doesn't drop what it took",
    file: "src/lower/drops.rs",
    find: "        for (place, ty) in places {\n            self.drop_in(place, ty, span, made, out)?;\n        }",
    replace: "        let _ = places;",
    tests: ["test/corpus.test.ts", "-t", "closure_drops"],
  },
  {
    name: "closure-drop-reversed",
    breaks:
      "a closure dropped uncalled drops what it took last first, not in the order it took them",
    file: "src/lower/drops.rs",
    find: "        for (place, ty) in places {",
    replace: "        for (place, ty) in places.into_iter().rev() {",
    tests: ["test/corpus.test.ts", "-t", "closure_drops"],
  },
  {
    name: "drops-itself-unasked",
    breaks:
      "a type inside itself, through an `Rc` or a map's entries, has its drop written in place forever, which overflows rust-js's stack (#305)",
    file: "src/lower/drops.rs",
    find: "            && (self.drops_itself(ty) || self.drop_size(ty, &mut Vec::new()) > 8)\n",
    replace: "            && self.drop_size(ty, &mut Vec::new()) > 8\n",
    tests: [
      "test/corpus.test.ts",
      "-t",
      "^(rc_counts|trie_of_counted_rcs).rs$",
    ],
  },
  {
    name: "hoisted-drops-redeclared",
    breaks:
      "a drop function of a trait's default method is declared again for each drop, without end for a type inside itself",
    file: "src/lower/drops.rs",
    find: "            .and_then(|hoisted| hoisted.functions.iter().find(|(t, _)| *t == ty))\n",
    replace:
      "            .and_then(|hoisted| hoisted.functions.iter().find(|_| false))\n",
    tests: ["test/corpus.test.ts", "-t", "default_method_drops"],
  },
  {
    name: "consumed-closed-required",
    breaks:
      "a generic `repeat(t).take(n)` takes no destructor of `T`, though `take`'s drop drops it",
    file: "src/lower/drops.rs",
    find: "        if self.recognition().closed_on_drop(output).is_none() {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^repeat_drops.rs$"],
  },
  {
    name: "default-drops-asked-in-trait",
    breaks:
      "a copied default's type in its impl's terms, `Buffer<SIZE>`, is asked about where the trait's method is, which hasn't `SIZE`, and rustc crashes",
    file: "src/lower/drops.rs",
    find: "        if let Some(typing_env) = self.impl_typing_env() {\n",
    replace:
      "        if let Some(typing_env) = None::<ty::TypingEnv<'tcx>> {\n",
    tests: ["test/corpus.test.ts", "-t", "^const_param_drops.rs$"],
  },
  {
    name: "drop-function-quiet-unknown",
    breaks:
      "a call of a drop function made here whose body never throws is taken as one that may, each part after it in a `finally` of its own (#416)",
    file: "src/lower/drops.rs",
    find: "        if self.drop_state.quiet.contains(name) {\n",
    replace: "        if false && self.drop_state.quiet.contains(name) {\n",
    tests: ["test/corpus.test.ts", "-t", "^nested_generic_types.rs$"],
    snapshots: true,
  },
];
