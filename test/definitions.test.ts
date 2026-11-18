import { expect, test } from "bun:test";
import { chmodSync, copyFileSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { newest, PRERELEASE_FOR } from "../scripts/definitions";
import { underLoad } from "../scripts/timing";
import { fixture, root } from "./support";
import { run } from "./child";

const now = Date.parse("2026-11-15T00:00:00Z");

// Stable first: a beta or a release candidate only where a capability no
// stable release supplies asks it (`PRERELEASE_FOR`), each with its test.
test("definition updates retain release channels and the three-day cool-off", () => {
  const tags = { latest: "7.0.3", beta: "7.1.0-beta", rc: "7.0.1-rc", next: "8.0.0-dev.1" };
  const times = {
    "7.0.2": "2026-11-10T00:00:00Z",
    "7.0.3": "2026-11-14T00:00:00Z",
    "7.1.0-beta": "2026-11-14T00:00:00Z",
    "7.0.1-rc": "2026-11-10T00:00:00Z",
    "8.0.0-dev.1": "2026-11-01T00:00:00Z",
    "9.0.0-insiders.1": "2026-11-01T00:00:00Z",
    "8.0.0-alpha.1": "2026-11-01T00:00:00Z",
    modified: "2026-11-15T00:00:00Z",
  };
  expect(newest(tags, times, now)).toBe("7.0.2");
  // An old enough beta is newer, but stable comes first.
  expect(newest(tags, { ...times, "7.1.0-beta": "2026-11-12T00:00:00Z" }, now)).toBe("7.0.2");
  // Where a capability asks it, the beta is eligible too.
  expect(newest(tags, { ...times, "7.1.0-beta": "2026-11-12T00:00:00Z" }, now, true)).toBe("7.1.0-beta");
  // And each capability that asks one names its test.
  for (const asked of Object.values(PRERELEASE_FOR).flat()) expect(asked.test).toMatch(/^test\/[\w-]+\.test\.ts -t /);
  expect(newest({ latest: "7.0.3" }, times, now)).toBe("7.0.2");
  expect(() => newest(tags, { "7.0.3": times["7.0.3"] }, now)).toThrow("No eligible release");
});

test("the parser and root consume the same engine release without a separate ES library package", () => {
  const rootPackage = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
  const parserPackage = JSON.parse(readFileSync(join(root, "typescript/package.json"), "utf8"));
  expect(parserPackage.dependencies.typescript).toBe(rootPackage.devDependencies.typescript);
  expect(rootPackage.devDependencies).not.toHaveProperty("typescript-es");
});


test("definition regeneration reports both libraries and retains a lost-member baseline", async () => {
  await Promise.all([[false, "OPEN"], [true, "OPEN"], [false, "CLOSED"]].map(async ([lost, state]) => {
    const closed = state === "CLOSED";
    const dir = fixture("definitions-orchestration");
    for (const name of ["scripts", "bin", "typescript", "webapi/src", "builtins"]) mkdirSync(join(dir, name), {recursive:true});
    copyFileSync(join(root, "scripts/definitions.ts"), join(dir, "scripts/definitions.ts"));
    writeFileSync(join(dir, "builtins/gaps.ts"), "export const gaps = () => [{ pattern: 'unbound', issue: 7 }];\n");
    writeFileSync(join(dir, "package.json"), JSON.stringify({devDependencies:{typescript:"7.0.2"}}, null, 2));
    writeFileSync(join(dir, "typescript/package.json"), JSON.stringify({dependencies:{typescript:"7.0.2"}}, null, 2));
    writeFileSync(join(dir, "webapi/package.json"), JSON.stringify({devDependencies:Object.fromEntries(["@types/web", "@webref/idl", "@webref/elements", "@webref/events", "webidl2"].map(name => [name,"1.0.0"]))}, null, 2));
    writeFileSync(join(dir, "webapi/src/lib.rs"), "pub struct Before;\n");
    for (const name of ["webapi", "builtins"]) {
      writeFileSync(join(dir, name, "coverage.txt"), "existing\n");
      writeFileSync(join(dir, name, "coverage.ts"), "export const measure = async () => ({all:['existing','added','unbound'],covered:" + JSON.stringify(lost ? ["added"] : ["existing","added"]) + "});\nexport const render = value => value.covered.join('\\n') + '\\n';\n");
    }
    const commands = join(dir, "commands.jsonl");
    const fake = join(dir, "bin/bun");
    writeFileSync(fake, "#!" + process.execPath + "\n" +
      "import {appendFileSync,writeFileSync} from 'node:fs';\n" +
      "const args=process.argv.slice(2);\nappendFileSync(" + JSON.stringify(commands) + ",JSON.stringify(args)+'\\n');\n" +
      "if(args[0]==='pm' && args[1]==='view'){const version=args[2]==='typescript'?'7.0.3':'1.0.0';console.log(JSON.stringify(args[3]==='dist-tags'?{latest:version}:{[version]:'2020-01-01T00:00:00Z'}));}\n" +
      "if(args[0]==='run' && args[1]==='generate')writeFileSync(" + JSON.stringify(join(dir,"webapi/src/lib.rs")) + ",'pub struct After;\\n');\n");
    chmodSync(fake, 0o755);
    const gh = join(dir, "bin/gh");
    writeFileSync(gh, "#!" + process.execPath + "\n" +
      "import {appendFileSync} from 'node:fs';\n" +
      "appendFileSync(" + JSON.stringify(commands) + ",JSON.stringify(['gh',...process.argv.slice(2)])+'\\n');\nconsole.log(" + JSON.stringify(state) + ");\n");
    chmodSync(gh, 0o755);
    const result = await run([process.execPath, join(dir,"scripts/definitions.ts")], dir, 10_000, {PATH:join(dir,"bin")+":"+process.env.PATH});
    expect(result.timedOut).toBe(false);
    expect(result.code, result.stdout + result.stderr).toBe(lost || closed ? 1 : 0);
    if (closed) expect(result.stdout).toContain("FAIL builtins/gaps.txt: #7 is closed");
    expect(JSON.parse(readFileSync(join(dir,"package.json"),"utf8")).devDependencies.typescript).toBe("7.0.3");
    expect(JSON.parse(readFileSync(join(dir,"typescript/package.json"),"utf8")).dependencies.typescript).toBe("7.0.3");
    for (const name of ["webapi", "builtins"]) {
      expect(result.stdout).toContain(name + " unbound:");
      expect(readFileSync(join(dir,name,"coverage.txt"),"utf8")).toBe(lost ? "existing\n" : "existing\nadded\n");
    }
    const called: string[][] = readFileSync(commands,"utf8").trim().split("\n").map(line=>JSON.parse(line));
    for (const generator of ["typed_arrays","errors","intl"]) expect(called).toContainEqual(["builtins/"+generator+".ts"]);
    expect(called).toContainEqual(["scripts/react-in-rust.ts","regen"]);
    // Each gap's issue asked open, and a stable release where a prerelease
    // was asked said removable.
    expect(called).toContainEqual(["gh","issue","view","7","--json","state","--jq",".state"]);
    expect(result.stdout).toContain("typescript 7.0.3 is stable: if `bun test test/definitions.test.ts");
    expect(called.some(args=>args[0]==="test" && args.includes("test/intl-arguments.test.ts"))).toBe(true);
    expect(result.stdout).toContain("types added (1): After");
    expect(result.stdout).toContain("types removed (1): Before");
  }));
});

// TypeScript is pinned at a beta, as `PRERELEASE_FOR` asks, while no stable
// release's DOM library declares what webapi's generator reads: these, and
// the event maps of the targets among them. Each missing fails here, as
// 7.0.2's did, so the prerelease can't outlive its reason (#1903).
test("the pinned TypeScript's DOM library declares what webapi reads", async () => {
  const { open } = await import("@rust-js/typescript");
  const session = await open([], { lib: ["esnext", "dom"] });
  const declared = new Set<string>();
  try {
    for (const file of await session.libraries()) for (const d of (await session.read(file)).declarations) if ("name" in d) declared.add(d.name);
  } finally {
    await session.close();
  }
  const read = [
    "CSSFontFaceDescriptors", "CloseWatcher", "CloseWatcherOptions", "DocumentPictureInPicture",
    "DocumentPictureInPictureEvent", "DocumentPictureInPictureEventInit", "DocumentPictureInPictureOptions",
    "HighlightsFromPointOptions", "Origin", "ParseHTMLUnsafeOptions", "Serial", "SerialOptions",
    "SerialOutputSignals", "SerialPort", "SerialPortFilter", "SerialPortRequestOptions", "SetHTMLOptions",
    "SetHTMLUnsafeOptions", "WebTransportReceiveStream", "WebTransportSendStream",
    "CloseWatcherEventMap", "DocumentPictureInPictureEventMap", "SerialPortEventMap",
  ];
  expect(read.filter((name) => !declared.has(name))).toEqual([]);
}, underLoad(60_000));
