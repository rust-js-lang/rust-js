// What a mutant's build is held to, built incrementally (scripts/mutant-builds.ts).
import type { Mutation } from "../mutations";
const tests = ["test/mutations.test.ts", "-t", "a mutant's build is its mutation's"];
export const mutations: Mutation[] = [
  {
    name: "mutant-build-unrebuilt-taken",
    breaks: "a build cargo left as it was, a mutant's that its incremental cache said was done, is taken as compiled again, and judged as the mutation",
    file: "scripts/mutant-builds.ts",
    find: "return { binary: message.executable, rebuilt: message.fresh === false };",
    replace: "return { binary: message.executable, rebuilt: true };",
    tests,
  },
  {
    name: "mutant-build-unit-binary-mistaken",
    breaks: "a unit-tested mutant's tests run the compiler's binary, not its unit tests', which runs no test of it",
    file: "scripts/mutant-builds.ts",
    find: "if (message.target?.name !== target || (message.profile?.test === true) !== unit) continue;",
    replace: "if (message.target?.name !== target) continue;",
    tests,
  },
  {
    name: "mutant-build-unwritten-file-taken",
    breaks: "a mutant's file not in the crate as written is built and judged as the mutation",
    file: "scripts/mutant-builds.ts",
    find: "if (!existsSync(join(crate, path)) || readFileSync(join(crate, path), \"utf8\") !== text) return",
    replace: "if (!existsSync(join(crate, path))) return",
    tests,
  },
  {
    name: "mutant-build-crate-as-it-is-taken",
    breaks: "a mutant's file the crate already has, so the build is the compiler as it is, is judged as the mutation",
    file: "scripts/mutant-builds.ts",
    find: "if (existsSync(join(source, path)) && readFileSync(join(source, path), \"utf8\") === text) return `${path} is as the crate has it`;",
    replace: "",
    tests,
  },
  {
    name: "fresh-verdict-of-its-text",
    breaks: "a fresh build's verdict is held to its incremental build's by the whole result, so two survivors whose logs differ in name differ",
    file: "scripts/mutant-builds.ts",
    find: "return result.split(/[\\s:]/)[0];",
    replace: "return result;",
    tests,
  },
  {
    name: "fresh-share-unchecked",
    breaks: "`--fresh=5` rebuilds every build fresh, not refused as no share",
    file: "scripts/mutant-builds.ts",
    find: "if (!(share >= 0 && share <= 1)) throw",
    replace: "if (Number.isNaN(share)) throw",
    tests,
  },
];
