// Mutations of src/lower/library/reached.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "mir-bodys-calls-unreached",
    breaks: "a library's generic function the crate compiles itself, of MIR alone, reaches nothing it names: its `Rc<Vec<u8>>`, given its library's trait's generic method that counts it, is uncounted, `strong_count` `NaN` (#1217)",
    file: "src/lower/library/reached.rs",
    find: "            if body.root().is_none()\n                && let Some(mir) = &body.mir\n            {\n                let named",
    replace: "            if false\n                && let Some(mir) = &body.mir\n            {\n                let named",
    tests: ["test/crates.test.ts", "-t", "two crates: library_rc_counted_through_a_traits_generic_method$"],
  },
  {
    name: "library-closure-unreached",
    breaks:
      "a library's `impl FnMut` of a value with a destructor, made a `dyn FnMut` by a crate that is a library itself, whose callers it can't see, is taken to drop nothing",
    file: "src/lower/library/reached.rs",
    find: "            ) => reached.functions |= library(def_id),\n",
    replace: "            ) => {}\n",
    tests: [
      "test/cargo-link.test.ts",
      "-t",
      "a library gives a crate's dyn Fns a drop only through its own functions",
    ],
  },
  {
    name: "library-dyn-unreached",
    breaks:
      "a `Box<dyn FnMut>` a library's function gives, of its own closure with a destructor, is taken to drop nothing",
    file: "src/lower/library/reached.rs",
    find: "                reached.fn_dyns_of.insert(library_key(tcx, krate));\n",
    replace: "",
    tests: [
      "test/cargo-link.test.ts",
      "-t",
      "a library gives a crate's dyn Fns a drop only through its own functions",
    ],
  },
  {
    name: "library-reached-any",
    breaks:
      "a crate using a library of plain items, a `bool`'s, takes each `dyn Fn` to drop and each `Rc` to be counted, and refuses React's scheduler",
    file: "src/lower/library/reached.rs",
    find: "        let mut reached = Reached::default();\n        if !self.any() {\n            return reached;\n        }\n",
    replace:
      "        let mut reached = Reached::default();\n        if self.any() {\n            return Reached { functions: true, fn_dyns_of: self.dependencies.libraries.keys().cloned().collect(), ..Reached::default() };\n        }\n",
    tests: [
      "test/cargo-link.test.ts",
      "-t",
      "a library gives a crate's dyn Fns a drop only through its own functions",
    ],
  },
  {
    name: "crate-closure-taken-for-the-librarys",
    breaks:
      "what a closure of the crate's given to a library's generic binding holds, an `Rc<RefCell<Vec<&dyn Fn(..)>>>`, is taken for a `dyn Fn` the library hands it, so every `dyn Fn` drops and an unrelated `Rc<RefCell<Vec<Rc<dyn Fn()>>>>` is counted, `{ value, strong, weak }` (ADR 0626, amended)",
    file: "src/lower/library/reached.rs",
    find: "                        && id.is_local()\n",
    replace: "                        && false\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "a crate's closure given to a library's binding hands the crate nothing",
    ],
  },
  {
    name: "merging-function-hands-its-rcs",
    breaks:
      "a library's function the crate compiles itself as it merges what the crate compares is taken to hand the crate its uncounted `Rc<dyn Fn>`s, refused where the crate counts them (#1052)",
    file: "src/lower/library/reached.rs",
    find: "                if !self.merged_here(id, args) {\n",
    replace: "                if true {\n",
    tests: ["test/crates.test.ts", "-t", "two crates: library_identities_kept_apart"],
  },
];
