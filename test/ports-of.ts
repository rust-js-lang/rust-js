// The ports in test/ports, compiled for the tests that read them, a module of its own: in test/support.ts, which every test
// imports, its reads of test/ports made each test a reader of them, so the
// selector ran the whole suite for a change to them (ADR 1101, amended).

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { buildNode } from "./node-crate";
import { compiler, fixture, root, run } from "./support";
import { withoutBoundary } from "./without-boundary";

const portsJs = new Map<string, string>();
/** A port in `test/ports/<port>` (React's, as react-in-rust ports it, copied
 * with the modules it uses), compiled once a run against the js, node and
 * webapi crates: the directory its modules' JS and declarations are in,
 * each by its file. */
export function portedDir(port: string): string {
  if (!portsJs.has(port)) {
    const out = fixture(port);
    const node = buildNode();
    const crates = ["js", "node", "webapi"].flatMap((lib) => ["--extern", `${lib}=${join(node, `lib${lib}.rmeta`)}`]);
    run([compiler, join(root, "test/ports", port, "lib.rs"), "-o", join(out, "lib.js"), "--declarations", "--", ...crates, "-L", node]);
    portsJs.set(port, out);
  }
  return portsJs.get(port)!;
}

/** A port's JS (`portedDir`), of each of its modules, by file: as it reads
 * without the boundary of what JS gives Rust, as a test of another feature
 * reads it (#2403); a `.d.ts` as it is. */
export function ported(port: string): (file: string) => string {
  const dir = portedDir(port);
  return (file) => {
    const text = readFileSync(join(dir, file), "utf8");
    return file.endsWith(".js") ? withoutBoundary(text) : text;
  };
}

/** React's shared/ReactPerformanceTrackProperties, as react-in-rust ports
 * it (`test/ports/performance-track-properties`): its JS, a function of it
 * at a time. */
export function performanceTrack(): (name: string) => string {
  const js = ported("performance-track-properties")("ReactPerformanceTrackProperties.js");
  return (name) => {
    const at = js.indexOf(`function ${name}(`);
    const ends = ["\nfunction ", "\nexport function ", "\nconst "].map((e) => js.indexOf(e, at + 1)).filter((e) => e > 0);
    return js.slice(at, Math.min(js.length, ...ends));
  };
}
