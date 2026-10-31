// Mutations of src/lower/bindings/given_names.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "alias-cased-as-own",
    breaks:
      "a use's alias of an item its rust_js::name names keeps Rust's case, `IdlePriority as IDLE_SCHEDULER_PRIORITY` (rd79)",
    file: "src/lower/bindings/given_names.rs",
    find: "    (super::given_name(tcx, def_id).and_then(|given| cased_like(&given, alias)))",
    replace:
      "    (super::given_name(tcx, def_id).and_then(|given| cased_like(&given, alias)).filter(|_| false))",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "rust_js::name names is cased",
    ],
  },
  {
    name: "alias-pascal-camel",
    breaks:
      "a use's alias of a PascalCase item is camelCase, `IdlePriority as idleSchedulerPriority`",
    file: "src/lower/bindings/given_names.rs",
    find: "        (true, _, true) => Some(crate::lower::upper_first(&camel)),\n",
    replace: "        (true, _, true) => Some(camel),\n",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "rust_js::name names is cased",
    ],
  },
  {
    name: "alias-screaming-cased",
    breaks: "a use's alias of a SCREAMING item is camelCase, `MAX_LANE as top`",
    file: "src/lower/bindings/given_names.rs",
    find: "        return Some(name.to_string());\n    }\n    // Its words",
    replace:
      "        return Some(crate::lower::variable_case(name));\n    }\n    // Its words",
    tests: [
      "test/cross-crate-items.test.ts",
      "-t",
      "rust_js::name names is cased",
    ],
  },
];
