// Invoked by Cargo's build.rs. Nothing generated here belongs in source control.
import { mkdirSync, readFileSync, writeFileSync, statSync, realpathSync, rmSync, readdirSync } from "node:fs";
import { resolve, relative, isAbsolute } from "node:path";
import { createHash } from "node:crypto";

const root = realpathSync(resolve(import.meta.dir, ".."));
const engine = realpathSync(resolve(root, "vendor/pocketjs"));
const outputArg = process.argv.find(a => a.startsWith("--outdir="));
if (!outputArg) throw new Error("UI generation requires --outdir=<Cargo output directory>; use cargo build --locked");
const output = resolve(outputArg.slice("--outdir=".length));
const version = readFileSync(resolve(root, ".bun-version"), "utf8").trim();
if (Bun.version !== version) throw new Error(`UI build requires Bun ${version}, found ${Bun.version}; see .bun-version`);
const hash = (path: string) => createHash("sha256").update(readFileSync(path)).digest("hex");
function uiInputs(directory: string): Record<string, string> {
  return Object.fromEntries(readdirSync(directory, { withFileTypes: true }).sort((a,b)=>a.name.localeCompare(b.name)).flatMap(entry => {
    const path = resolve(directory, entry.name);
    return entry.isDirectory() ? Object.entries(uiInputs(path)) : [[relative(root, path), hash(path)]];
  }));
}
// Cargo notices missing output via rerun-if-changed, but also sees freshly
// generated files as newer than the build's start. Recheck content here so that
// follow-up invocation does not reinstall dependencies or recompile the UI.
// The complete UI tree also detects added resources/importable modules.
const outputs = ["main.js", "main.pak", "manifest.json", "cargo-inputs.txt"];
try {
  const cache = JSON.parse(readFileSync(resolve(output, "cache.json"), "utf8"));
  if (JSON.stringify(cache.ui) === JSON.stringify(uiInputs(resolve(root, "ui")))
      && Object.entries(cache.inputs).every(([path, value]) => hash(path) === value)
      && outputs.every(name => hash(resolve(output, name)) === cache.outputs[name])) {
    console.log("Pocket Openworld UI: generated inputs and outputs are unchanged");
    process.exit(0);
  }
} catch { /* Missing/changed inputs or outputs require generation. */ }

async function run(args: string[], cwd: string) {
  const child = Bun.spawn([process.execPath, ...args], { cwd, stdout: "inherit", stderr: "inherit" });
  if (await child.exited !== 0) throw new Error(`UI build command failed: bun ${args[0]}`);
}

// Let Bun check the pinned dependency graph; a fresh clone needs no manual
// install or application node_modules symlinks. Warm installs use Bun's cache.
await run(["install", "--frozen-lockfile"], engine);
mkdirSync(output, { recursive: true });
for (const name of [...outputs, "inputs.json", "cache.json"])
  rmSync(resolve(output, name), { force: true });
await run([
  resolve(engine, "tools/build.ts"), resolve(root, "ui/main.tsx"),
  "--framework=solid", "--no-config", "--density=2", `--outdir=${output}`,
  `--inputs-file=${resolve(output, "inputs.json")}`,
], root);

const inside = (base: string, path: string) => {
  const name = relative(base, path);
  return name !== ".." && !name.startsWith("../") && !name.startsWith("..\\") && !isAbsolute(name);
};
const paths: string[] = JSON.parse(readFileSync(resolve(output, "inputs.json"), "utf8"));
paths.push(...["build.rs", ".bun-version", "tools/build_ui.ts"].map(p => resolve(root, p)));
const files = [...new Set(paths.map(p => realpathSync(p)))].filter(p => statSync(p).isFile()).sort();
const sources: Record<string, string> = {}, engineSources: Record<string, string> = {};
for (const path of files) {
  if (/[\r\n]/.test(path)) throw new Error("newline in build dependency path");
  if (inside(engine, path)) {
    const name = relative(engine, path).replaceAll("\\", "/");
    if (!name.startsWith("node_modules/") && !name.startsWith(".cache/")) engineSources[name] = hash(path);
  } else if (inside(root, path) && !inside(output, path)) {
    sources[relative(root, path).replaceAll("\\", "/")] = hash(path);
  }
}
// Track individual files. The compiler also reports directories for optional
// lockfiles; watching the engine root would include caches and cause rebuilds
// on every invocation. Cargo separately watches the application UI directory.
writeFileSync(resolve(output, "cargo-inputs.txt"), files.join("\n") + "\n");
writeFileSync(resolve(output, "manifest.json"), JSON.stringify({
  schema: 1, bunVersion: Bun.version, sources, engineSources,
  artifacts: Object.fromEntries(["main.js", "main.pak"].map(p => [p, hash(resolve(output, p))])),
}, null, 2) + "\n");
writeFileSync(resolve(output, "cache.json"), JSON.stringify({
  ui: uiInputs(resolve(root, "ui")),
  inputs: Object.fromEntries(files.map(path => [path, hash(path)])),
  outputs: Object.fromEntries(outputs.map(name => [name, hash(resolve(output, name))])),
}, null, 2) + "\n");
