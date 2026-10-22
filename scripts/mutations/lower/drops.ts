// Mutations of src/lower/drops.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "drop-function-in-branch",
    breaks: "a drop function of a trait's default method is declared where it's first needed, a branch another call isn't in",
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
    breaks: "a closure dropped uncalled drops what it took last first, not in the order it took them",
    file: "src/lower/drops.rs",
    find: "        for (place, ty) in places {",
    replace: "        for (place, ty) in places.into_iter().rev() {",
    tests: ["test/corpus.test.ts", "-t", "closure_drops"],
  },
  {
    name: "rc-drop-size-unseen",
    breaks: "a type inside itself through an `Rc` drops itself forever",
    file: "src/lower/drops.rs",
    find: "                let (size, recursive) = self.drop_size(pointee, stack);\n",
    replace: "                let (size, recursive) = (1, false);\n",
    tests: ["test/corpus.test.ts", "-t", "rc_counts"],
  },
  {
    name: "hoisted-drops-redeclared",
    breaks: "a drop function of a trait's default method is declared again for each drop, without end for a type inside itself",
    file: "src/lower/drops.rs",
    find: "            .and_then(|hoisted| hoisted.functions.iter().find(|(t, _)| *t == ty))\n",
    replace: "            .and_then(|hoisted| hoisted.functions.iter().find(|_| false))\n",
    tests: ["test/corpus.test.ts", "-t", "default_method_drops"],
  },
  {
    "name": "plain-refcell-bound-kept",
    "breaks": "a field's `RefCell` a pattern binds, a cell of its own, is taken for its value",
    "file": "src/lower/drops.rs",
    "find": "                    self.found.insert(field.pattern.ty);\n",
    "replace": "",
    "tests": ["test/lowering-cells.test.ts", "-t", "only borrowed for a moment"]
  },
  {
    name: "consumed-closed-required",
    breaks: "a generic `repeat(t).take(n)` takes no destructor of `T`, though `take`'s drop drops it",
    file: "src/lower/drops.rs",
    find: "        if self.recognition().closed_on_drop(output).is_none() {\n",
    replace: "        if true {\n",
    tests: ["test/corpus.test.ts", "-t", "^repeat_drops.rs$"],
  },
  {
    name: "default-drops-asked-in-trait",
    breaks: "a copied default's type in its impl's terms, `Buffer<SIZE>`, is asked about where the trait's method is, which hasn't `SIZE`, and rustc crashes",
    file: "src/lower/drops.rs",
    find: "        if let Some(typing_env) = self.impl_typing_env() {\n",
    replace: "        if let Some(typing_env) = None::<ty::TypingEnv<'tcx>> {\n",
    tests: ["test/corpus.test.ts", "-t", "^const_param_drops.rs$"],
  },
];
