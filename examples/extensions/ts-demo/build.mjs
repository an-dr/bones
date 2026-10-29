// Builds dist/ts_demo.wasm: bundles the TypeScript into one ES module, which
// componentize-js needs, then componentizes it against wit/extension.wit.

import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync } from "node:fs";
import { join } from "node:path";
import { build } from "esbuild";

const here = import.meta.dirname;
mkdirSync(join(here, "dist"), { recursive: true });

await build({
  entryPoints: [join(here, "src/extension.ts")],
  bundle: true,
  format: "esm",
  // The host's imports are resolved by the component model, not a file.
  external: ["bones:extension/*"],
  outfile: join(here, "build/extension.js"),
  logLevel: "warning",
});

// jco's command-line entry, wherever this version keeps it. Read from the
// package directly, because jco's exports do not include its manifest.
const jcoDir = join(here, "node_modules/@bytecodealliance/jco");
const { bin } = JSON.parse(readFileSync(join(jcoDir, "package.json"), "utf8"));
const jco = join(jcoDir, typeof bin === "string" ? bin : bin.jco);
execFileSync(process.execPath, [
  jco, "componentize", join(here, "build/extension.js"),
  "--wit", join(here, "../../../wit/extension.wit"),
  "--world-name", "extension",
  // The engine links WASI's CLI interfaces but not its HTTP ones.
  "--disable", "http", "fetch-event",
  "-o", join(here, "dist/ts_demo.wasm"),
], { stdio: "inherit" });
