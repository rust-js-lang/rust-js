// Mutations of src/runtime/rc.js (ADR 0320).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "rc-clone-uncounted",
    breaks: "a clone of a counted `Rc` isn't counted",
    file: "src/runtime/rc.js",
    find: "function $rcClone(rc) {\n  rc.strong++;\n",
    replace: "function $rcClone(rc) {\n",
    tests: ["test/corpus.test.ts", "-t", "rc_counts"],
  },
  {
    name: "rc-drop-never-last",
    breaks: "the last `Rc` doesn't drop what it points at",
    file: "src/runtime/rc.js",
    find: "  if (--rc.strong === 0) {\n",
    replace: "  if (--rc.strong === -1) {\n",
    tests: ["test/corpus.test.ts", "-t", "rc_counted_uses"],
  },
  {
    name: "upgrade-dangling",
    breaks: "a `Weak` upgrades after the last `Rc` is gone",
    file: "src/runtime/rc.js",
    find: "  if (weak.strong === 0) return undefined;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "rc_counts"],
  },
  {
    name: "weak-count-dangling",
    breaks: "a `Weak`'s `weak_count()` counts with nothing strong left",
    file: "src/runtime/rc.js",
    find: "  return weak.strong === 0 ? 0 : weak.weak;\n",
    replace: "  return weak.weak;\n",
    tests: ["test/corpus.test.ts", "-t", "rc_counts"],
  },
  {
    name: "make-mut-shares",
    breaks: "`make_mut` of a shared `Rc` changes what the others point at",
    file: "src/runtime/rc.js",
    find: "  return { value: clone ? clone(rc.value) : rc.value, strong: 1, weak: 0 };\n",
    replace: "  return { value: rc.value, strong: 1, weak: 0 };\n",
    tests: ["test/corpus.test.ts", "-t", "rc_counted_uses"],
  },
  {
    name: "try-unwrap-shared",
    breaks: "`try_unwrap` of a shared `Rc` takes its value",
    file: "src/runtime/rc.js",
    find: "  if (rc.strong !== 1) return { TAG: \"Err\", _0: rc };\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "rc_counted_uses"],
  },
  {
    name: "new-cyclic-weak-kept",
    breaks: "`new_cyclic`'s own `Weak` is still counted after",
    file: "src/runtime/rc.js",
    find: "  rc.weak--;\n",
    replace: "",
    tests: ["test/corpus.test.ts", "-t", "rc_counted_uses"],
  },
];
