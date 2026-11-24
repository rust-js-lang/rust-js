// Mutations of crates/rust-js-lower/src/session/shadows.rs (ADR 1199).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "shadow-lines-of-no-kind",
    breaks: "a shadow's lines are written without the kind they're of, so one of several kinds can't be told from another (ADR 1199)",
    file: "crates/rust-js-lower/src/session/shadows.rs",
    find: "            report.push_str(kind.name());\n            report.push('\\t');\n",
    replace: "",
    tests: ["test/drops-shadow.test.ts", "-t", "each cleanup verdict of the partial moves"],
  },
  {
    name: "shadow-kind-unasked-recorded",
    breaks: "`--shadow` records no kind it's asked, so its file is empty (ADR 1199)",
    file: "crates/rust-js-lower/src/session/shadows.rs",
    find: "    if let Some(lines) = shadows.lines.borrow_mut().get_mut(&kind) {\n",
    replace: "    if let Some(lines) = shadows.lines.borrow_mut().get_mut(&kind).filter(|_| false) {\n",
    tests: ["test/cargo.test.ts", "-t", "RUST_JS_SHADOW"],
  },
];
