// Mutations of src/lower/bindings/value_only.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "value-only-called",
    breaks:
      "a call of a binding read as a value alone, a window's `setTimeout` given a `Box<dyn FnOnce()>`, compiles, a second way to set a timer (rd314, ADR 1020)",
    file: "src/lower/bindings/value_only.rs",
    find: "            if let Some(message) = value_only(tcx, callee) {\n",
    replace: "            if let Some(message) = value_only(tcx, callee).filter(|_| false) {\n",
    tests: ["test/js-timers.test.ts", "-t", "window.set_timeout is gone"],
  },
];
