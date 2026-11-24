// Mutations of crates/rust-js-lower/src/reexports/js_reached.rs (#2403
// lever (3)): what JS may call, so what's checked.
import type { Mutation } from "../../../mutations";

const file = "crates/rust-js-lower/src/reexports/js_reached.rs";
const tests = (title: string) => ["test/boundary-reach.test.ts", "-t", title];

export const mutations: Mutation[] = [
  {
    name: "js-reached-reexported-function-unsettled",
    breaks: "a function the root's `pub use` gives JS is called unreachable, so JS calls `given` unchecked (#2403 lever (3))",
    file,
    find: "            DefKind::Fn => {\n                self.functions.insert(id);\n            }\n",
    replace: "            DefKind::Fn => {}\n",
    tests: tests("one its pub use gives, are checked"),
  },
  {
    name: "js-reached-namespace-unfollowed",
    breaks: "a module whose namespace the root re-exports, `pub use shown as display`, is called unreachable, so JS calls its functions unchecked (#2403 lever (3))",
    file,
    find: "        pending.extend(reached.modules.iter().copied());\n",
    replace: "",
    tests: tests("whose namespace the root re-exports"),
  },
  {
    name: "js-reached-forwarder-target-unreached",
    breaks: "a forwarder's target, which its module re-exports in the forwarder's place, is called unreachable, so JS calls it unchecked through that re-export (rd575, #2403 lever (3))",
    file,
    find: "            if more.is_empty() {\n",
    replace: "            if true {\n",
    tests: tests("a forwarder's target is checked"),
  },
  {
    name: "js-reached-module-unseen",
    breaks: "a function of a module JS may call is called unreachable by its module, so the root's own function is unchecked (#2403 lever (3))",
    file,
    find: "        self.modules.contains(&tcx.parent_module_from_def_id(item))\n",
    replace: "        false\n",
    tests: tests("the root's own function"),
  },
  {
    name: "class-unexported-by-visibility",
    breaks: "a class's members that only another crate's re-export gives JS aren't checked, as no wrapper keeps its `instanceof` (#2403 (Y))",
    file: "crates/rust-js-lower/src/reexports/js_reached.rs",
    find: " || (class.is_local() && crate::library::reachable(tcx, class))",
    replace: "",
    tests: ["test/cross-crate-items.test.ts", "-t", "a pub use of a library's item re-exports it from the library's file"],
  },
];
