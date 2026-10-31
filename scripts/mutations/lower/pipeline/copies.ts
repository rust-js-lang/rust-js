// Mutations of src/lower/pipeline/copies.rs (ADR 0093).
import type { Mutation } from "../../../mutations";

export const mutations: Mutation[] = [
  {
    name: "js-given-copy-kept",
    breaks:
      "a copy made alike its function is kept, so `echo` is two functions, `echoBoxed` beside it",
    file: "src/lower/pipeline/copies.rs",
    find: "            if let Some((to, _)) = found {",
    replace: "            if let Some((to, _)) = found.filter(|_| false) {",
    tests: ["test/corpus.test.ts", "-t", "js_given_copies"],
    snapshots: true,
  },
  {
    name: "js-given-copy-self-call-apart",
    breaks:
      "a copy that calls itself is read as calling a copy, never alike its function, so `count` is two functions, `countUnboxed` beside it",
    file: "src/lower/pipeline/copies.rs",
    find: "                as_if.insert(named(*id, Some(*at)), named(*id, settled(&same, *id, *to)).2);",
    replace: "                let _ = to;",
    tests: ["test/corpus.test.ts", "-t", "js_given_copies"],
    snapshots: true,
  },
  {
    name: "namespace-copy-unrenamed",
    breaks:
      "a copy of a function read through its module's namespace, `Helpers.wrappedUnboxed`, keeps the name of the copy it was alike, so it calls a copy the module doesn't export (rd114, ADR 1018)",
    file: "src/lower/pipeline/copies.rs",
    find: "                        let owner = (symbol.export != js::NAMESPACE).then(|| symbol.export.clone());\n",
    replace:
      "                        let owner = Some(symbol.export.clone());\n",
    tests: ["test/corpus.test.ts", "-t", "^js_given_copies.rs$"],
    snapshots: true,
  },
  {
    name: "through-copy-unlike-allowed",
    breaks:
      "a namespace's method called through a value whose call needs a copy unlike the module's own JS calls that JS, which reads `Some(None)` as `None` (rd311)",
    file: "src/lower/pipeline/copies.rs",
    find: "        if !alike.contains(&(id, at)) {\n",
    replace: "        if false {\n",
    tests: ["test/namespaces.test.ts", "-t", "unlike its own JS"],
  },
  {
    name: "through-copy-alike-refused",
    breaks:
      "a namespace's method called through a value whose call needs a copy alike the module's own JS, `batchedUpdates`, is refused (rd311)",
    file: "src/lower/pipeline/copies.rs",
    find: "        if !alike.contains(&(id, at)) {\n",
    replace: "        if true {\n",
    tests: ["test/namespaces.test.ts", "-t", "unlike its own JS"],
  },
];
