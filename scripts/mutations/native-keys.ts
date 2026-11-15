// A native program's key (test/support.ts's `nativeKey`, ADR 0567 amended).
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "native-key-dependencies-unread",
    breaks: "a native program built against another build of the crates `-L dependency=` names is the kept one",
    file: "test/support.ts",
    find: '    else if (flag.startsWith("dependency=/") && existsSync(path) && statSync(path).isDirectory()) hash.update(`${portable(path)} ${dependencies(path)}`);\n',
    replace: "",
    tests: ["test/native-outcomes.test.ts", "-t", "dependencies' directory"],
  },
];
