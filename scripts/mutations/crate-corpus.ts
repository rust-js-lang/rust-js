// Mutations of scripts/crate-corpus.ts: what the crate corpus measures and
// holds each crate to.
import type { Mutation } from "../mutations";
export const mutations: Mutation[] = [
  {
    name: "crate-labels-unlisted-pass",
    breaks: "a label MIR made in the crate corpus's JS that isn't listed under #2215 passes the crate corpus's check, unseen, as std's did before the label rule read it (#2215)",
    file: "scripts/crate-corpus.ts",
    find: "    if (!listed.has(label)) problems.push(",
    replace: "    if (false) problems.push(",
    tests: ["test/ci.test.ts", "-t", "the crate corpus's labels MIR made are listed"],
  },
  {
    name: "crate-snapshot-unheld",
    breaks: "a snapshotted function of a real crate's JS that moved passes the crate corpus's check, as itertools' `putBack` keeping an unread drop flag did unseen (#1979)",
    file: "scripts/crate-corpus.ts",
    find: "    if (was === snapshot) continue;\n",
    replace: "    if (was !== undefined) continue;\n",
    tests: ["test/ci.test.ts", "-t", "the crate corpus's snapshotted functions are cut whole and held"],
  },
  {
    name: "crate-js-naming-the-home-passes",
    breaks: "a crate's JS that names the builder's `$CARGO_HOME`, its own panic locations', passes the crate corpus's check, the home shipped in an app's JS unseen (#2435)",
    file: "scripts/crate-corpus.ts",
    find: "  return homes.some((home) => text.includes(`${home}/`));\n",
    replace: "  return false;\n",
    tests: ["test/ci.test.ts", "-t", "the crate corpus's snapshotted functions are cut whole and held"],
  },
  {
    name: "crate-js-naming-the-checkout-passes",
    breaks: "a crate's JS that names rust-js's checkout, a panic in its own js-sys', passes the crate corpus's check, the builder's home shipped in an app's JS unseen (#2438)",
    file: "scripts/crate-corpus.ts",
    find: "export function namesMachine(text: string, homes = [cargoHome, checkout]): boolean {\n",
    replace: "export function namesMachine(text: string, homes = [cargoHome]): boolean {\n",
    tests: ["test/ci.test.ts", "-t", "the crate corpus's snapshotted functions are cut whole and held"],
  },
  {
    name: "crate-snapshot-cut-of-a-longer-name",
    breaks: "a snapshotted function is cut where another's name begins with its, `putBackDebug` for `putBack`",
    file: "scripts/crate-corpus.ts",
    find: "(?:function\\\\*? |class |const |let )${name.replace(/\\$/g, \"\\\\$\")}\\\\b`);\n",
    replace: "(?:function\\\\*? |class |const |let )${name.replace(/\\$/g, \"\\\\$\")}`);\n",
    tests: ["test/ci.test.ts", "-t", "the crate corpus's snapshotted functions are cut whole and held"],
  },
  {
    name: "crate-snapshot-unlisted-kept",
    breaks: "a crate's snapshot no entry of `snapshotted` lists stays, unread, after its entries are gone",
    file: "scripts/crate-corpus.ts",
    find: ".filter((path) => !files.has(path) && measured(path.split(\"@\")[0]!))",
    replace: ".filter((path) => !files.has(path) && false)",
    tests: ["test/ci.test.ts", "-t", "the crate corpus's snapshotted functions are cut whole and held"],
  },
  {
    name: "crate-project-held-listed",
    breaks: "the prune lists a crate's project a run is measuring, so its cap's bound removes it under Cargo and rust-js: itertools' JS gone mid-measure in check:fast, 3 of 3 on the M6 (#2562)",
    file: "scripts/crate-corpus.ts",
    find: "    .filter((path) => statSync(path, { throwIfNoEntry: false })?.isDirectory() && !isHeld(path))",
    replace: "    .filter((path) => statSync(path, { throwIfNoEntry: false })?.isDirectory())",
    tests: ["test/support.test.ts", "-t", "a crate project being measured is never removed under its run"],
  },
];
