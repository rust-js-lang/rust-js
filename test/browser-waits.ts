// What the tests that drive a browser through Vite wait on (#1429): each
// wait a condition, with a deadline as long as the machine's load makes it
// (`underLoad`), whose timeout says what didn't happen and what the page
// showed instead; Vite's watcher polling, and ready before a page opens.
//
// Watching by macOS's file system events, Vite heard a test's save tens of
// seconds late under load, or not before the deadline, and heard files the
// test wrote just before the server started, `index.html`, a module rust-js
// build wrote, as changes beside its later save: a full reload, or a second
// update Fast Refresh can't keep state through (`Clicks 0`).

import { rmSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import type { Locator } from "@playwright/test";
import type { ViteDevServer } from "vite";
import { underLoad } from "../scripts/timing";

/** How a test's Vite server watches its files: by polling, each file's
 * modification time read every 100 ms, which sees a write however far
 * behind macOS's file system events run under load, where they came tens
 * of seconds late, or not before the deadline, and which reports nothing a
 * test wrote before the server started (#1429). */
export const polled = { usePolling: true, interval: 100 } as const;

/** How long a browser wait may take on this machine now: 30 s at rest. */
export function browserDeadline(): number {
  return underLoad(30_000);
}

/** `locator` seen, within `browserDeadline`: else an error that names what
 * it was waiting for, Playwright's own description of it, and what the
 * page's body showed then. */
export async function seen(locator: Locator): Promise<void> {
  const deadline = browserDeadline();
  try {
    await locator.waitFor({ timeout: deadline });
  } catch (error) {
    const shown = await locator
      .page()
      .locator("body")
      .innerText({ timeout: 1_000 })
      .catch(() => "(the page couldn't be read)");
    throw new Error(`${locator} wasn't seen within ${deadline} ms; the page showed:\n${shown.slice(0, 2_000)}\n${error}`);
  }
}

/** `server`'s watcher ready: a file written in `root`, and its event
 * awaited, within `browserDeadline`, the file then removed, so a test's
 * first save is heard. Written again every half second until it's
 * reported: `listen()` may return before the watcher's first scan has
 * ended, which reports no file it finds, so one written once then is
 * never heard. */
export async function watcherSettled(server: ViteDevServer, root: string): Promise<void> {
  const sentinel = join(root, `watcher-settled-${process.pid}-${Date.now()}.txt`);
  const deadline = browserDeadline();
  const reported = new Promise<void>((resolve, reject) => {
    const timer = setTimeout(
      () => reject(new Error(`Vite's watcher didn't report ${sentinel} within ${deadline} ms`)),
      deadline,
    );
    const heard = (_event: string, file: string) => {
      if (file === sentinel) {
        clearTimeout(timer);
        server.watcher.off("all", heard);
        resolve();
      }
    };
    server.watcher.on("all", heard);
  });
  let writes = 0;
  const write = () => writeFileSync(sentinel, String(writes++));
  write();
  const again = setInterval(write, 500);
  try {
    await reported;
  } finally {
    clearInterval(again);
    rmSync(sentinel, { force: true });
  }
}
