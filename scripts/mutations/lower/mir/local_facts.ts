// Mutations of src/lower/mir/local_facts.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "quiet-with-borrow-of-another-module",
    breaks: "another module's quiet thread-local's `with_borrow` is given its value where it's lowered, not as that module's import reads it (ADR 0866, amended)",
    file: "src/lower/mir/local_facts.rs",
    find: "                        && tcx.parent_module_from_def_id(key) == self.module =>\n",
    replace: "                        =>\n",
    tests: ["test/lowering-module-variables.test.ts"],
  },
  {
    name: "facts-held-arms-unjoined",
    breaks: "a guard that holds still tests in each alternative of its arm, its graph made without the arms it goes on to (ADR 0790)",
    file: "src/lower/mir/local_facts.rs",
    find: "            graph: cfg::Graph::of(body, &held_arms, &payload_tests),\n",
    replace: "            graph: cfg::Graph::of(body, &Default::default(), &payload_tests),\n",
    tests: ["test/lowering-labels.test.ts", "-t", "a block the program labeled that ends a case"],
  },
  {
    name: "facts-patterned-none",
    breaks: "a temporary a `let` pattern of one field takes apart is folded, `const getProps = f().get_props`",
    file: "src/lower/mir/local_facts.rs",
    find: "            patterned: self.mir_patterned(body),\n",
    replace: "            patterned: Default::default(),\n",
    tests: ["test/lowering-matches.test.ts", "-t", "pattern of one field"],
  },
  {
    name: "facts-jsx-props-none",
    breaks: "a component's props with a flattened field are flattened before JSX takes them apart",
    file: "src/lower/mir/local_facts.rs",
    find: "            jsx_props: self.mir_jsx_props(body),\n",
    replace: "            jsx_props: Default::default(),\n",
    tests: ["test/jsx-props.test.ts"],
  },
  {
    name: "quiet-field-calls-unseen",
    breaks: "no call of a struct's quiet field is found, so each is checked by its type's borrows elsewhere (ADR 0863, amended)",
    file: "src/lower/mir/local_facts.rs",
    find: "                    quiet.calls.insert(block);\n                    if matches!(known, Std::Borrow { .. })",
    replace: "                    if matches!(known, Std::Borrow { .. })",
    tests: ["test/lowering-field-cells.test.ts", "-t", "whatever else has its type"],
  },
  {
    name: "module-variable-set-unseen",
    breaks: "a thread-local that's its module's variable is set as a cell, `$borrowMut(current)` (ADR 0270, amended)",
    file: "src/lower/mir/local_facts.rs",
    find: "                        quiet.calls.insert(block);\n                    }\n                    if crate::lower::bindings::nullable_local(tcx, key) {\n",
    replace: "                    }\n                    if crate::lower::bindings::nullable_local(tcx, key) {\n",
    tests: ["test/lowering-field-cells.test.ts", "-t", "whatever else has its type"],
  },
  {
    name: "quiet-guards-unmoved",
    breaks: "a quiet field's guard moved to a variable, `let times = root.times.borrow(); .. drop(times);`, releases what it never counted, `$unborrow(times)` (ADR 0863, amended)",
    file: "src/lower/mir/local_facts.rs",
    find: "            quiet.guards.extend(more);\n",
    replace: "            let _ = more;\n            break;\n",
    tests: ["test/lowering-field-cells.test.ts", "-t", "whatever else has its type"],
  },
  {
    name: "nullable-local-calls-unseen",
    breaks: "no call that writes a nullable thread-local is found, so its `None` is written `undefined` (ADR 0586, amended)",
    file: "src/lower/mir/local_facts.rs",
    find: "                        quiet.nullable.insert(block);\n",
    replace: "",
    tests: ["test/lowering-module-variables.test.ts", "-t", "nullable thread-local's None is null"],
  },
];
