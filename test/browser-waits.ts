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

import { readFileSync, rmSync, writeFileSync } from "node:fs";
import { join, relative } from "node:path";
import type { Locator, Page } from "@playwright/test";
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

/** What a test's Vite server heard and its page did, each at its time
 * since the trace began: the watcher's events of the files under `root`,
 * the Vite client's own lines, `[vite] hot updated: /src/App.jsx`, and each
 * load of the page. A wait for a save's update that runs out says them
 * (`applied`): whether the save went unheard, or was heard and never
 * applied (#1529). */
export class Trace {
  private readonly start = Date.now();
  private readonly lines: string[] = [];

  constructor(server: ViteDevServer, page: Page, private readonly root: string) {
    server.watcher.on("all", (event: string, file: string) => this.add(`watcher ${event} ${relative(root, file)}`));
    page.on("console", (message) => {
      if (message.text().startsWith("[vite]")) this.add(`page ${message.text()}`);
    });
    page.on("load", () => this.add("page loaded"));
  }

  private add(line: string): void {
    this.lines.push(`${Date.now() - this.start} ms ${line}`);
  }

  /** How many lines it has: where a wait's own begin. */
  get length(): number {
    return this.lines.length;
  }

  /** Whether the watcher has reported `file` since line `from`. */
  heard(file: string, from: number): boolean {
    const event = ` ${relative(this.root, file)}`;
    return this.lines.slice(from).some((line) => line.includes(" watcher ") && line.endsWith(event));
  }

  toString(): string {
    return this.lines.slice(-60).join("\n");
  }
}

/** `locator` seen after a save whose build wrote `written`, each a path:
 * within a third of `browserDeadline`; else each written again, its bytes
 * as they are and its time new, which Vite hears as another save, at most
 * three times. Its timeout says whether the watcher heard the last writes,
 * and what the server and the page did meanwhile (`trace`): a save
 * unheard, or one heard and never applied (#1529). */
export async function applied(locator: Locator, written: string[], trace: Trace): Promise<void> {
  const deadline = browserDeadline() / 3;
  for (let tries = 1; ; tries++) {
    const from = trace.length;
    if (tries > 1) for (const file of written) writeFileSync(file, readFileSync(file));
    try {
      await locator.waitFor({ timeout: deadline });
      return;
    } catch (error) {
      if (tries < 3) continue;
      const unheard = written.filter((file) => !trace.heard(file, from));
      const which = unheard.length > 0
        ? `Vite's watcher never heard ${unheard.join(", ")}: the save went unheard`
        : "Vite's watcher heard the save, but the page never showed its update";
      throw new Error(`${locator} wasn't seen after ${tries} writes of ${written.join(", ")}: ${which}. What the server and the page did:\n${trace}\n${error}`);
    }
  }
}

/** `locator`'s element made live by React, within `browserDeadline`: a
 * page a server rendered shows it before its JS hydrates it, and a click
 * then does nothing, or a link's loads the page again. Its sign is the
 * props React gives each element it renders or hydrates
 * (`__reactProps$<id>`, its key on the DOM node) (#1529). */
export async function hydrated(locator: Locator): Promise<void> {
  const deadline = browserDeadline();
  const element = await locator.elementHandle({ timeout: deadline });
  try {
    await locator.page().waitForFunction(
      (node) => node !== null && Object.keys(node).some((key) => key.startsWith("__reactProps$")),
      element,
      { timeout: deadline },
    );
  } catch (error) {
    throw new Error(`${locator} wasn't hydrated by React within ${deadline} ms\n${error}`);
  }
}

/** `button` clicked once React has made it live (`hydrated`), then
 * `shown` seen. A page loaded again after the click, by a dev server's
 * full reload, its state gone, is clicked again, at most three times; and
 * the timeout says which it was (#1529). */
export async function clicked(button: Locator, shown: Locator): Promise<void> {
  const page = button.page();
  const deadline = browserDeadline();
  for (let tries = 1; ; tries++) {
    await hydrated(button);
    await page.evaluate(() => ((window as any).clickedHere = true));
    await button.click();
    const end = Date.now() + deadline;
    let reloaded = false;
    while (Date.now() < end) {
      if (await shown.isVisible()) return;
      reloaded = await page.evaluate(() => (window as any).clickedHere !== true).catch(() => true);
      if (reloaded) break;
      await Bun.sleep(250);
    }
    if (reloaded && tries < 3) continue;
    const which = reloaded
      ? "the page was loaded again after each click, its state gone"
      : "React had hydrated the button and the page wasn't loaded again, but the click's update never showed";
    const body = await page.locator("body").innerText({ timeout: 1_000 }).catch(() => "(the page couldn't be read)");
    throw new Error(`${shown} wasn't seen within ${deadline} ms of ${tries} clicks: ${which}; the page showed:\n${body.slice(0, 2_000)}`);
  }
}
