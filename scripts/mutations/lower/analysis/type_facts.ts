// Mutations of src/lower/analysis/type_facts.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "type-facts-not-passed-on",
    breaks: "a generic function passing its `U` on to one that asks `size_of::<T>()` isn't given `U`'s size",
    file: "src/lower/analysis/type_facts.rs",
    find: "                if asked.contains(&(callee, to, fact)) && asked.insert((caller, from, fact)) {",
    replace: "                if false && asked.contains(&(callee, to, fact)) && asked.insert((caller, from, fact)) {",
    tests: ["test/corpus.test.ts", "-t", "type_facts"],
  },
  {
    name: "type-facts-unsized-asked",
    breaks: "`size_of_val` of an unsized `T` asks its caller for a size its type hasn't",
    file: "src/lower/analysis/type_facts.rs",
    find: "                    && of.is_sized(tcx, typing_env)\n",
    replace: "\n",
    tests: ["test/diagnostics.test.ts", "-t", "size_of_val of an unsized"],
  },
  {
    name: "dyn-any-param-unasked",
    breaks: "a `T: 'static` made a `dyn Any` has no `TypeId` to give it",
    file: "src/lower/analysis/type_facts.rs",
    find: "                asked.insert((caller, param.index, TypeFact::Id));\n                continue;",
    replace: "                continue;",
    tests: ["test/corpus.test.ts", "-t", "dyn_any_generic"],
  },
  {
    name: "any-bound-fact-asked",
    breaks: "a `T: Any` is given its `TypeId` beside its dictionary",
    file: "src/lower/analysis/type_facts.rs",
    find: "                    && !(fact == TypeFact::Id && bound_by_any(tcx, caller, param.index))",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "^dyn_any.rs"],
    snapshots: true,
  },
];
