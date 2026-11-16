// Mutations of src/js/imports.rs (ADR 0093).
import type { Mutation } from "../../mutations";

export const mutations: Mutation[] = [
  {
    name: "namespace-import-not-a-local",
    breaks:
      "a module's namespace import, `import * as Plugin`, isn't among the names its imports bind, so what's spelled apart from them, or reclaimed, doesn't see it (ADR 1018, ADR 1150)",
    file: "src/js/imports.rs",
    find: "        self.named.iter().map(|(_, local)| local).chain(&self.namespace)\n",
    replace: "        self.named.iter().map(|(_, local)| local).chain(None)\n",
    tests: ["test/corpus.test.ts", "-t", "^module_path_namespace.rs$"],
    snapshots: true,
  },
];
