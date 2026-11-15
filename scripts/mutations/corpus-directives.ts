// The corpus's directives (test/corpus-cases.ts): `//@ nondeterministic`.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "nondeterministic-directive-unread",
    breaks: "`//@ nondeterministic` is read as nothing, so a case whose run reads the world has its run kept",
    file: "test/corpus-cases.ts",
    find: '    else if (name === "nondeterministic" && value) why = value;\n',
    replace: '    else if (name === "nondeterministic" && value) {}\n',
    tests: ["test/native-outcomes.test.ts", "-t", "says it's nondeterministic"],
  },
];
