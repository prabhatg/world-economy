// The engine in its own thread, so the page stays responsive during a run of minutes.
// One worker per run: stopping a run terminates it, and its memory goes with it.

import { runUnit } from "./engine";

export type ToWorker = { module: WebAssembly.Module; unit: object; packs: Map<string, Uint8Array> };
export type FromWorker =
  | { kind: "progress"; years: number }
  | { kind: "done"; result: object; files: Map<string, Uint8Array> }
  | { kind: "error"; message: string };

self.onmessage = async (e: MessageEvent<ToWorker>) => {
  const post = (m: FromWorker) => self.postMessage(m);
  try {
    const { module, unit, packs } = e.data;
    const run = await runUnit(module, unit, packs, (years) => post({ kind: "progress", years }));
    post({ kind: "done", result: run.result, files: run.files });
  } catch (err) {
    post({ kind: "error", message: err instanceof Error ? err.message : String(err) });
  }
};
