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
  {
    name: "unchanged-build-written-in-place",
    breaks: "what's built from changed inputs is written where another test run reads it, so a JSX test found `target/libreact.rmeta` gone as a corpus part's run wrote it (check 44024336584)",
    file: "test/support.ts",
    find: "    work(into);\n",
    replace: "    work(dir);\n",
    tests: ["test/harness.test.ts", "-t", "what's built from unchanged inputs isn't built again"],
  },
];
