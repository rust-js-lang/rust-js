// Mutations of src/prepare/undefined_keys.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "led-none-always-kept",
    breaks: "a `None` a comment on lines of its own is before keeps its key where a key follows, so `// We'll initialize these at the root.` writes `_currentValue: undefined` and not `_currentValue2`",
    file: "src/prepare/undefined_keys.rs",
    find: "            let kept = !undefined(p) || (led && !kept_after);\n",
    replace: "            let kept = !undefined(p) || led;\n",
    tests: ["test/lowering-comment-fields.test.ts", "-t", "a comment before a field changes no key but where it has no other place"],
  },
  {
    name: "led-none-last-left-out",
    breaks: "a last `None` a comment on lines of its own is before is left out, and the comment is before the object",
    file: "src/prepare/undefined_keys.rs",
    find: "            let kept = !undefined(p) || (led && !kept_after);\n",
    replace: "            let kept = !undefined(p) || (led && false);\n",
    tests: ["test/lowering-comment-fields.test.ts", "-t", "a comment before a field changes no key but where it has no other place"],
  },
  {
    name: "commented-none-left-out",
    breaks: "a `None` field a comment is written on is left out, and its comment with it, `v: undefined, // why` gone",
    file: "src/prepare/undefined_keys.rs",
    find: " && !layout.commented.contains(key))",
    replace: ")",
    tests: ["test/lowering-comment-fields.test.ts", "-t", "a None field a comment is written on keeps its key and its comment"],
  },
  {
    name: "undefined-fields-kept",
    breaks: "`{ code, hidden: true, read_only: undefined }`, a `None`'s key",
    file: "src/prepare/undefined_keys.rs",
    find: "    if props.iter().any(|p| matches!(p, Prop::Spread(_))) {\n",
    replace: "    if true {\n",
    tests: ["test/lowering-options.test.ts", "-t", "literal None is left out"],
  },
  {
    name: "undefined-after-spread-dropped",
    breaks: "`{ ...base, read_only: undefined }` is `{ ...base }`, base's `Some`",
    file: "src/prepare/undefined_keys.rs",
    find: "    if props.iter().any(|p| matches!(p, Prop::Spread(_))) {\n",
    replace: "    if false {\n",
    tests: ["test/lowering-options.test.ts", "-t", "literal None is left out"],
  },
];
