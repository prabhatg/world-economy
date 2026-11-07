// Runs a work unit in the engine the gate checks: the `oikos` CLI built for
// wasm32-wasip1, under a WASI shim whose file system lives in memory. The run
// directory holds the unit and everything it writes; the packs sit read-only at
// /packs, outside it. Results are bit-identical to native hosts (docs/03 §3.5).

import { ConsoleStdout, Directory, File, type Inode, OpenFile, PreopenDirectory, WASI, WASIProcExit } from "@bjorn3/browser_wasi_shim";
import { parseExact, stringifyExact } from "./json";

export interface WorkResult {
  state_hash: string;
  summaries?: { calendar_year: number; population: number }[];
  [key: string]: unknown;
}

export interface Run {
  result: WorkResult;
  /** The run directory: relative path → bytes. */
  files: Map<string, Uint8Array>;
}

function collect(dir: Directory, prefix: string, out: Map<string, Uint8Array>): void {
  for (const [name, node] of dir.contents as Map<string, Inode>) {
    if (node instanceof Directory) collect(node, `${prefix}${name}/`, out);
    else if (node instanceof File) out.set(prefix + name, node.data);
  }
}

/** Runs `unit` (its pack paths already at `/packs`) with `packs` (id → bytes);
 * `onProgress` gets the years done, about once per century. */
export async function runUnit(
  module: WebAssembly.Module,
  unit: object,
  packs: Map<string, Uint8Array>,
  onProgress?: (years: number) => void,
): Promise<Run> {
  const errors: string[] = [];
  const stderr = ConsoleStdout.lineBuffered((line) => {
    const m = /^\{"years": (\d+), "of": \d+\}$/.exec(line);
    if (m) onProgress?.(Number(m[1]));
    else errors.push(line);
  });
  const run = new PreopenDirectory(".", new Map([["unit.json", new File(new TextEncoder().encode(stringifyExact(unit)))]]));
  const packDir = new PreopenDirectory(
    "/packs",
    new Map([...packs].map(([id, bytes]) => [`${id}.oikospack`, new File(bytes, { readonly: true })])),
  );
  const fds = [new OpenFile(new File([])), ConsoleStdout.lineBuffered(() => {}), stderr, run, packDir];
  const wasi = new WASI(["oikos", "run", "unit.json", "-o", "result.json", "--progress"], [], fds);
  const instance = await WebAssembly.instantiate(module, { wasi_snapshot_preview1: wasi.wasiImport });
  let code: number;
  try {
    code = wasi.start(instance as unknown as Parameters<WASI["start"]>[0]);
  } catch (e) {
    if (!(e instanceof WASIProcExit)) throw e;
    code = e.code;
  }
  if (code !== 0) throw new Error(errors.join("\n") || `the engine exited with code ${code}`);
  const files = new Map<string, Uint8Array>();
  collect(run.dir, "", files);
  const result = files.get("result.json");
  if (!result) throw new Error("the engine wrote no result.json");
  return { result: parseExact(new TextDecoder().decode(result)) as WorkResult, files };
}
