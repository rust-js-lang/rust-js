// React's scheduler, its DOM fork, as react-in-rust ports it
// (`test/ports/scheduler`): the JS rust-js writes of each of its modules,
// against the snapshot in `test/snapshots/ports/scheduler`, so a change to
// any of it is this test's failure, not a port's surprise. Review each
// change against upstream's `Scheduler.js` and `SchedulerMinHeap.js`;
// `BLESS=1` writes the snapshot.

import { test } from "bun:test";
import { join } from "node:path";
import { expectSnapshot, portedDir, root } from "./support";

// Each file's header names its source from the repository's root, the same
// on every machine.
test("the scheduler's JS and declarations are its snapshot's", () => {
  const normalize = (text: string) => text.replaceAll(`${root}/`, "");
  expectSnapshot(portedDir("scheduler"), join(root, "test/snapshots/ports/scheduler"), { normalize, declarations: true });
}, 600_000);
