// Mutations of src/prepare/reclaim.rs (ADR 0093).
import type { Mutation } from "../../mutations";

const tests = ["test/lowering.test.ts", "-t", "nothing it shadows"];

export const mutations: Mutation[] = [
  {
    name: "reclaim-shadowing-read",
    breaks: "a local is named as the module's function it reads, which it then shadows",
    file: "src/prepare/reclaim.rs",
    find: " || mentioned.contains(base)\n    {",
    replace: "\n    {",
    tests,
  },
  {
    name: "reclaim-keyword",
    breaks: "a parameter named `class` in Rust is `class` in JS, a word JS keeps",
    file: "src/prepare/reclaim.rs",
    find: "    if crate::names::js_ident(base) != base && !GLOBALS.contains(&base) {",
    replace: "    if false && crate::names::js_ident(base) != base && !GLOBALS.contains(&base) {",
    tests,
  },
  {
    name: "reclaim-no-globals",
    breaks: "an import named apart from `Error` stays `Error$` where nothing reads the global",
    file: "src/prepare/reclaim.rs",
    find: "    if crate::names::js_ident(base) != base && !GLOBALS.contains(&base) {",
    replace: "    if crate::names::js_ident(base) != base {",
    tests,
  },
  {
    name: "reclaim-conversion-unread",
    breaks: "a local is `String` where a conversion, `String(n)`, reads the global",
    file: "src/prepare/reclaim.rs",
    find: "        ExprKind::Stringed(_) if reads => f(&mut \"String\".to_string()),",
    replace: "        ExprKind::Stringed(_) if false => f(&mut \"String\".to_string()),",
    tests,
  },
  {
    name: "reclaim-on-load-locals",
    breaks: "two `on_load!` bodies' `x`s, the module's, are each `x`",
    file: "src/prepare/reclaim.rs",
    find: "        if !matches!(item, js::Item::Statements(_)) {",
    replace: "        if true {",
    tests: ["test/lowering.test.ts", "-t", "on_load bodies that bind one name each"],
  },
  {
    name: "reclaim-no-imports",
    breaks: "an import named apart from a global nothing reads keeps its `$`",
    file: "src/prepare/reclaim.rs",
    find: "    imports(module);\n",
    replace: "",
    tests,
  },
  {
    name: "reclaim-no-locals",
    breaks: "a local named apart from a module's name its function never reads keeps its `$1`",
    file: "src/prepare/reclaim.rs",
    find: "            locals(item);\n",
    replace: "",
    tests,
  },
  {
    name: "reclaim-no-lower-suffix",
    breaks: "`result$2` stays, after `result$1` became `result`, where it could be `result$1`",
    file: "src/prepare/reclaim.rs",
    find: "                    .chain((1..suffix).map(|i| format!(\"{base}${i}\")))",
    replace: "                    .chain((1..1).map(|i| format!(\"{base}${i}\")))",
    tests: ["test/snapshots.test.ts", "-t", "calc"],
    snapshots: true,
  },
];
