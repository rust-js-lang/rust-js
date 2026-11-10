// A setting of the environment that selects a part of a test file,
// `METAMORPHIC=all` say (scripts/shard.ts' `parts`), read here alone, so a
// run can say it was read. One that sets it and runs no file that reads it
// ran nothing it selects, and would pass as if it had: `METAMORPHIC=all bun
// test test/corpus.test.ts` ran the corpus, not a rewrite, and passed, which
// hid a P0 (#1210). test/setup.ts fails such a run as it ends.

import { readdirSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { parts } from "../scripts/shard";

/** The settings read so far in this process: of a run one after another,
 * every file's; of `--parallel`, the one file's. On `globalThis`, as the
 * files of one process may each load this module apart. */
const read: Set<string> = ((globalThis as { partSettingsRead?: Set<string> }).partSettingsRead ??= new Set());

/** The setting `name` of the environment, which must be one of `parts`',
 * noted as read. */
export function partSetting(name: string): string | undefined {
  if (!parts.some((part) => part.setting === name)) throw new Error(`${name} selects no part of scripts/shard.ts' \`parts\`: read it with process.env`);
  read.add(name);
  return process.env[name];
}

/** Each of `parts`' settings `env` sets that nothing in this process read. */
export function unreadSettings(env: Record<string, string | undefined> = process.env, wasRead: ReadonlySet<string> = read): string[] {
  const settings = [...new Set(parts.map((part) => part.setting))];
  return settings.filter((name) => env[name] !== undefined && env[name] !== "" && !wasRead.has(name));
}

/** The files of test/ that read `name`, by their text: each that asks
 * `partSetting` and names it, `"NAME"`. */
export function readersOf(name: string): string[] {
  const named = new RegExp(`["']${name}["']`);
  return readdirSync(import.meta.dir)
    .filter((file) => /\.tsx?$/.test(file) && file !== "part-settings.ts")
    .filter((file) => {
      const text = readFileSync(join(import.meta.dir, file), "utf8");
      return text.includes("partSetting(") && named.test(text);
    })
    .map((file) => `test/${file}`);
}

/** Why a run fails that set `unread` and read none of them, or nothing. */
export function unreadMessage(unread: string[], env: Record<string, string | undefined> = process.env): string | undefined {
  if (unread.length === 0) return undefined;
  const each = unread.map((name) => `${name}=${env[name]} selects a part of ${readersOf(name).join(" or ")}, which this run didn't run`);
  return `${each.join("; ")}: run that file, or unset it, as nothing it selects ran (#1210)`;
}
