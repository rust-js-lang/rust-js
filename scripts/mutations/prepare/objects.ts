// Mutations of src/prepare/objects.rs (ADR 0093): an object literal as a
// person writes it.
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "entries-kept",
    breaks: "`Object.fromEntries([[\"/index.html\", code]])` stays a call where it's the object literal `{ \"/index.html\": code }`",
    file: "src/prepare/objects.rs",
    find: "    if object != \"Object\" || name != \"fromEntries\" {",
    replace: "    if true || object != \"Object\" || name != \"fromEntries\" {",
    tests: ["test/lowering-strings.test.ts", "-t", "a dictionary of entries written out"],
  },
  {
    name: "entries-proto-literal",
    breaks: "`Object.fromEntries([[\"__proto__\", x]])` is `{ __proto__: x }`, which sets the prototype",
    file: "src/prepare/objects.rs",
    find: "        if given || key == \"__proto__\" || matches!(value.kind, ExprKind::Undefined) {",
    replace: "        if given || matches!(value.kind, ExprKind::Undefined) {",
    tests: ["test/lowering-strings.test.ts", "-t", "a dictionary of entries written out"],
  },
  {
    name: "entries-twice-literal",
    breaks: "a key given twice is an object literal with it twice",
    file: "src/prepare/objects.rs",
    find: "        if given || key == \"__proto__\" || matches!(value.kind, ExprKind::Undefined) {",
    replace: "        if key == \"__proto__\" || matches!(value.kind, ExprKind::Undefined) {",
    tests: ["test/lowering-strings.test.ts", "-t", "a dictionary of entries written out"],
  },
  {
    name: "entries-undefined-literal",
    breaks: "an entry of `undefined` is a field an object leaves out",
    file: "src/prepare/objects.rs",
    find: "        if given || key == \"__proto__\" || matches!(value.kind, ExprKind::Undefined) {",
    replace: "        if given || key == \"__proto__\" {",
    tests: ["test/lowering-strings.test.ts", "-t", "a dictionary of entries written out"],
  },
  {
    name: "spread-written-out-kept",
    breaks: "React's `UIEventInterface` is `{ ...EventInterface, ...{ view: 0, detail: 0 } }`, where upstream writes `{...EventInterface, view: 0, detail: 0}` (ADR 0967, amended)",
    file: "src/prepare/objects.rs",
    find: "    if !props.iter().any(written_out) {\n",
    replace: "    if true || !props.iter().any(written_out) {\n",
    tests: ["test/js-collections.test.ts", "-t", "a Dict spread before more of it"],
  },
];
