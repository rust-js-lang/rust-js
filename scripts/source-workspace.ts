// The source-copy worker: the one owner of files, aliases and disk-floor checks.
import {constants,copyFileSync,existsSync,lstatSync,mkdirSync,readdirSync,readFileSync,statSync,symlinkSync,writeFileSync} from "node:fs";
import {dirname,join,relative} from "node:path";
import {parentPort,workerData} from "node:worker_threads";
import {bytesOf} from "../tooling/bounded.js";
import {floorBytes} from "../test/bounded";

/** An independently writable source tree, without generated stores. The
 * caller supplies Git's tracked files, including current working-tree edits.
 * Its workspace packages' links are relative, so a clone of the whole tree
 * (`makeSourceWorkspace`, #2503) resolves them inside itself; and it's
 * marked made of `files`, by `key`, with its size, so a clone of it knows
 * it's what a copy would be. */
function copySourceWorkspace(
  root: string,
  workspace: string,
  files: string[],
  free: number,
  key: string,
  marker: string,
): number {
  const kept = files.filter(
    (file) =>
      !file
        .split("/")
        .some((part) => ["target", "node_modules", ".git"].includes(part)),
  );
  const needed = kept.reduce(
    (total, file) => total + lstatSync(join(root, file)).blocks * 512,
    0,
  );
  if (free - needed < floorBytes)
    throw new Error(
      "a source mutation workspace would cross the development disk floor",
    );
  // Made fresh, it's no more than `files`: one copied over another's isn't
  // marked, as what was there before is in it too.
  const fresh = !existsSync(workspace);
  mkdirSync(workspace, { recursive: true });
  for (const file of kept) {
    const from = join(root, file),
      to = join(workspace, file);
    mkdirSync(dirname(to), { recursive: true });
    copyFileSync(from, to, constants.COPYFILE_FICLONE);
  }
  const manifest = JSON.parse(readFileSync(join(root, "package.json"), "utf8"));
  const packages = new Map<string, string>();
  for (const pattern of manifest.workspaces ?? []) {
    for (const directory of new Bun.Glob(pattern).scanSync({
      cwd: root,
      onlyFiles: false,
    })) {
      const file = join(root, directory, "package.json");
      if (existsSync(file))
        packages.set(JSON.parse(readFileSync(file, "utf8")).name, directory);
    }
  }
  const linkDependencies = (directory: string) => {
    const installed = join(root, directory, "node_modules");
    const local = join(workspace, directory, "node_modules");
    mkdirSync(local, { recursive: true });
    for (const name of existsSync(installed) ? readdirSync(installed) : []) {
      if (name === ".bin") continue;
      const source = join(installed, name);
      if (name.startsWith("@") && statSync(source).isDirectory()) {
        mkdirSync(join(local, name), { recursive: true });
        for (const child of readdirSync(source)) {
          if (!packages.has(name + "/" + child))
            symlinkSync(join(source, child), join(local, name, child));
        }
      } else if (!packages.has(name)) symlinkSync(source, join(local, name));
    }
    for (const [name, path] of packages) {
      const to = join(local, name);
      mkdirSync(dirname(to), { recursive: true });
      symlinkSync(relative(dirname(to), join(workspace, path)), to);
    }
  };
  linkDependencies("");
  for (const directory of packages.values()) linkDependencies(directory);
  const bytes = bytesOf(workspace);
  if (fresh) writeFileSync(join(workspace, marker), JSON.stringify({ key, bytes }));
  return bytes;
}

type Workspace = {root:string; workspace:string; files:string[]; free:number; key:string; marker:string};
const task = workerData as Workspace;
parentPort!.postMessage(copySourceWorkspace(task.root,task.workspace,task.files,task.free,task.key,task.marker));
parentPort!.close();
