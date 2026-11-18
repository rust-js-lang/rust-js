// Generator-chain owner mutations; preparation regenerates the canonical
// binding, then the named real caller test rebuilds its own js metadata.
import type { Mutation } from "../../mutations";
const tests = [
  "test/intl-arguments.test.ts",
  "-t",
  "pinned TypeScript argument unions",
];
const prepare = [process.execPath, "builtins/intl.ts"];
export const mutations: Mutation[] = [
  {
    name: "intl-locales-without-locale",
    breaks:
      "Intl locales stop admitting an Intl.Locale object accepted by the pinned declarations",
    file: "builtins/intl.ts",
    find: '"&String", "&Locale", "LocaleElement',
    replace: '"&String", "LocaleElement',
    prepare,
    tests,
  },
  {
    name: "intl-format-without-date",
    breaks:
      "formatting stops accepting Date objects from the native Date-or-number union",
    file: "builtins/intl.ts",
    find: 'const dateMembers = ["&Date", "f64"];',
    replace: 'const dateMembers = ["f64"];',
    prepare,
    tests,
  },
  {
    name: "intl-ranges-without-bigint",
    breaks:
      "format range arguments stop admitting the pinned native bigint form",
    file: "builtins/intl.ts",
    find: '[...dateMembers, "&BigInt", "i64", "u64"]',
    replace: "dateMembers",
    prepare,
    tests,
  },
  {
    name: "intl-supported-options-narrowed",
    breaks:
      "RelativeTimeFormat supportedLocalesOf loses the native full formatter options type",
    file: "builtins/intl.ts",
    find: '{ options: "RelativeTimeFormatOptions", supportedOptions: "constructor" }',
    replace: '{ options: "RelativeTimeFormatOptions" }',
    prepare,
    tests,
  },
  {
    name: "intl-constructors-run-unmarked",
    breaks: "a formatter's constructor `RUNS` says runs only what it's given, `new Intl.DateTimeFormat`, is written without `runs_only_given`, so what's committed of intl.rs isn't what it writes (#2082)",
    file: "builtins/intl.ts",
    find: "    attributesOf(`new ${js}`),\n",
    replace: "    [],\n",
    tests: ["test/builtins-coverage.test.ts", "-t", "Intl is what its template writes"],
  },
];
