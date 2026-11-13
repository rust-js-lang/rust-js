// Mutations of src/lower/copies/kept.rs, what a function React gives again copies (#1723).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "kept-changed-itself-uncopied",
    breaks: "a parameter React kept that a function changes in place itself isn't copied first (#1723)",
    file: "src/lower/copies/kept.rs",
    find: "                copies.at_start.insert(local);\n",
    replace: "",
    tests: ["test/react-kept.test.ts", "-t", "copied where it's changed"],
  },
  {
    name: "kept-moved-out-uncopied",
    breaks: "a part moved out of what React kept and changed isn't copied where it's moved (#1723)",
    file: "src/lower/copies/kept.rs",
    find: "            if changed(&held, true) {\n",
    replace: "            if changed(&held, true) && false {\n",
    tests: ["test/react-kept.test.ts", "-t", "copied where it's changed"],
  },
  {
    name: "kept-given-to-js-copied",
    breaks: "what's given on unchanged to React is copied, `<Moved at={{ ...props.at }} />`, where it needs none (#1723)",
    file: "src/lower/copies/kept.rs",
    find: "                            if !self.callee_is_js(body, func)\n",
    replace: "                            if true\n",
    tests: ["test/react-kept.test.ts", "-t", "copied where it's changed"],
  },
  {
    name: "kept-vec-items-shared",
    breaks: "a `Vec` React kept, its items changed, is copied by `slice()` alone, so its items are React's (#1723)",
    file: "src/lower/copies/kept.rs",
    find: "                return Ok(Expr::builtin(place, \"slice\", Vec::new()));\n",
    replace: "                return Ok(Expr::builtin(place, \"slice\", Vec::new()));\n            }\n            if true {\n                return Ok(Expr::builtin(place, \"slice\", Vec::new()));\n",
    tests: ["test/react-kept.test.ts", "-t", "copied where it's changed"],
  },
  {
    name: "kept-pattern-part-uncopied",
    breaks: "a part a parameter's pattern takes apart of what React kept, `{ at }`, isn't copied first (#1723)",
    file: "src/lower/copies/kept.rs",
    find: "        let copied = |param: &mir::Local| copies.at_start.contains(param) || copies.at_moves.contains(param);\n",
    replace: "        let copied = |_: &mir::Local| false;\n",
    tests: ["test/react-kept.test.ts", "-t", "copied where it's changed"],
  },
  {
    name: "kept-only-given-again",
    breaks: "every function's parameter given by value is kept, so a plain function copies what it owns (#1723)",
    file: "src/lower/copies/kept.rs",
    find: "            && self.krate.given_again.contains(&id)\n",
    replace: "",
    tests: ["test/corpus.test.ts"],
  },
  {
    name: "react-compiler-net-catches-a-skip",
    breaks: "a component that changes its props in place writes React's, which React Compiler refuses, \"This value cannot be modified\", and skips the component: the net catches it (ADR 1142, #1723)",
    file: "src/lower/copies/kept.rs",
    find: "            if !name.is_empty() && copy.kind != Expr::var(&name).kind {\n",
    replace: "            if false && copy.kind != Expr::var(&name).kind {\n",
    tests: ["test/react-compiler.test.ts", "-t", "compiles every component"],
  },
];
