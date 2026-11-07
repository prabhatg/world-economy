// Copies the engine the gate checks (engine/target/wasm32-wasip1/release/oikos.wasm)
// for the browser client, without its custom sections (debug info and names: 19 of its
// 21 MB). Code and data sections are copied byte for byte, so it computes the same.
//   node scripts/engine.mjs [out]    (default: dist/oikos.wasm)

import { existsSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

export const ENGINE = join(import.meta.dirname, "..", "..", "engine", "target", "wasm32-wasip1", "release", "oikos.wasm");

/** `wasm` without its custom sections (id 0). */
export function strip(wasm) {
  const b = new Uint8Array(wasm);
  if (b.length < 8 || b[0] !== 0 || b[1] !== 0x61 || b[2] !== 0x73 || b[3] !== 0x6d) throw new Error("not a wasm module");
  const keep = [b.subarray(0, 8)];
  let i = 8;
  while (i < b.length) {
    const start = i;
    const id = b[i++];
    let size = 0;
    for (let shift = 0; ; shift += 7) {
      const x = b[i++];
      size += (x & 0x7f) * 2 ** shift;
      if (x < 0x80) break;
    }
    i += size;
    if (i > b.length) throw new Error("truncated wasm module");
    if (id !== 0) keep.push(b.subarray(start, i));
  }
  const out = new Uint8Array(keep.reduce((n, k) => n + k.length, 0));
  keep.reduce((at, k) => (out.set(k, at), at + k.length), 0);
  return out;
}

if (process.argv[1] === import.meta.filename) {
  const out = process.argv[2] ?? join(import.meta.dirname, "..", "dist", "oikos.wasm");
  if (!existsSync(ENGINE)) {
    console.log("SKIP engine for the volunteer page: build it with cargo build --release --target wasm32-wasip1 -p oikos-cli");
  } else {
    mkdirSync(dirname(out), { recursive: true });
    writeFileSync(out, strip(readFileSync(ENGINE)));
  }
}
