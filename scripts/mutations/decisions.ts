// Mutations of src/decisions.rs (ADR 0093): decision coverage's counts.
import type { Mutation } from "../mutations";

export const mutations: Mutation[] = [
  {
    name: "decisions-never-counted",
    breaks: "a compile asked to count decisions counts none, every answer `0`, and the report says each never reached",
    file: "src/decisions.rs",
    find: "        count(decision, answer, \"-\");\n",
    replace: "        _ = (decision, answer);\n",
    tests: ["test/decisions.test.ts", "-t", "counts every answer"],
  },
  {
    name: "decisions-unasked-answers-unsaid",
    breaks: "a compile's counts leave out each answer it never gave, so the report can't tell one never reached from one no net asked",
    file: "src/decisions.rs",
    find: "                    if n > 0 || whole {\n",
    replace: "                    if n > 0 {\n",
    tests: ["test/decisions.test.ts", "-t", "counts every answer"],
  },
  {
    name: "empty-decisions-setting-counts",
    breaks: "an empty `RUST_JS_DECISIONS`, a workflow's of a run not the nightly, asks counts written to a file named ``, each compile warning it can't",
    file: "src/decisions.rs",
    find: "    LazyLock::new(|| std::env::var_os(\"RUST_JS_DECISIONS\").filter(|path| !path.is_empty()));\n",
    replace: "    LazyLock::new(|| std::env::var_os(\"RUST_JS_DECISIONS\"));\n",
    tests: ["test/decisions.test.ts", "-t", "empty setting"],
  },
  {
    name: "decisions-body-untold",
    breaks: "every decision is counted of the crate's analysis, `crate`, never of the body it was given in, its own or another crate's, so the report can't say an answer one never reaches",
    file: "src/decisions.rs",
    find: "    let outer = BODY.with(|body| body.replace(if own { \"own\" } else { \"other\" }));\n",
    replace: "    let outer = BODY.with(|body| body.replace(if own { \"crate\" } else { \"crate\" }));\n",
    tests: ["test/decisions.test.ts", "-t", "counts every answer"],
  },
];
