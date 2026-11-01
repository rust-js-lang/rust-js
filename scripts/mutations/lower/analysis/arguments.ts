// Mutations of src/lower/analysis/arguments.rs (rd332).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "arguments-unread",
    breaks: "a function that reads `js::arguments()` isn't one, so its calls leave out a trailing `undefined` and a closure that forwards to it is it, `new LinearGradient()` (rd332)",
    file: "src/lower/analysis/arguments.rs",
    find: "            None => _ = reading.insert(body.def_id),\n",
    replace: "            None => {}\n",
    tests: ["test/arguments.test.ts"],
  },
  {
    name: "arguments-in-closure-kept",
    breaks: "`js::arguments()` in a closure, an arrow whose `arguments` is the function's around it, is compiled (rd332)",
    file: "src/lower/analysis/arguments.rs",
    find: "    if tcx.is_closure_like(def_id) {\n",
    replace: "    if false && tcx.is_closure_like(def_id) {\n",
    tests: ["test/arguments.test.ts"],
  },
  {
    name: "arguments-in-generic-kept",
    breaks: "`js::arguments()` in a generic function, whose JS is given dictionaries too, is compiled (rd332)",
    file: "src/lower/analysis/arguments.rs",
    find: "    if generic {\n",
    replace: "    if false && generic {\n",
    tests: ["test/arguments.test.ts"],
  },
  {
    name: "arguments-in-trait-method-kept",
    breaks: "`js::arguments()` in a trait's method, called through its dictionary, is compiled (rd332)",
    file: "src/lower/analysis/arguments.rs",
    find: "    if tcx.trait_of_assoc(def_id).is_some() || tcx.trait_impl_of_assoc(def_id).is_some() {\n",
    replace: "    if false {\n",
    tests: ["test/arguments.test.ts"],
  },
  {
    name: "arguments-in-plain-method-kept",
    breaks: "`js::arguments()` in a method of what's no class, given the value first, is compiled (rd332)",
    file: "src/lower/analysis/arguments.rs",
    find: "        None if tcx.associated_item(def_id).is_method() => Some(\n",
    replace: "        None if false => Some(\n",
    tests: ["test/arguments.test.ts"],
  },
];
