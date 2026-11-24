// What snapshots.test.ts and snapshots-serde.test.ts share (#2465): a
// case's compile, from the repository's root, and its JS compared with its
// snapshot file by file (test/snapshots/).

import { test } from "bun:test";
import { rmSync } from "node:fs";
import { basename, join } from "node:path";
import { underLoad } from "../scripts/timing";
import { buildSerde } from "./serde-crate";
import { buildReact, compiler, expectSnapshot, root, run, target } from "./support";

export type Crate = "web" | "react" | "serde";

/** A test of each of `cases`: a name (its folder in test/snapshots/), the
 * crate's root, the crates it uses, and where its snapshot is, of one whose
 * JS an app commits beside its Rust, not under test/snapshots/. */
export function snapshotTests(cases: [string, string, Crate[], string?][]) {
  for (const [name, input, crates, committed] of cases) {
    test(`${name}: the generated JS is its snapshot`, () => {
      const out = join(target, "snapshots", name);
      rmSync(out, { recursive: true, force: true });
      const flags: string[] = [];
      if (crates.includes("web") || crates.includes("react")) buildReact();
      if (crates.includes("web")) flags.push("--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target);
      if (crates.includes("react")) flags.push("--extern", `react=${join(target, "libreact.rmeta")}`, "-L", target);
      if (crates.includes("serde")) flags.push(...buildSerde());
      // From the repository's root, so each file's header names its source the
      // same way on every machine.
      run([compiler, input, "-o", join(out, `${basename(input, ".rs")}.js`), ...(flags.length > 0 ? ["--", ...flags] : [])]);
      expectSnapshot(out, join(root, committed ?? join("test/snapshots", name)));
      // A compile of a crate of serde's, `inbox`'s, takes past bun's 5 s on a
      // loaded machine, the wasm job's too: its compile's time, not a hang.
      // Beside `check:fast`'s rebuilds of a change's ten mutations, its 15 s
      // alone took past 60 s, and at a load of 43 on 12 cores past 120 s:
      // longer as the machine's load is (`underLoad`).
    }, underLoad(120_000));
  }
}
