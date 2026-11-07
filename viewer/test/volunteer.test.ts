import { execFileSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { blake3 } from "@noble/hashes/blake3.js";
import { bytesToHex } from "@noble/hashes/utils.js";
import { describe, expect, it } from "vitest";
import { strip } from "../scripts/engine.mjs";
import { runUnit } from "../src/volunteer/engine";
import { type Engine, type PackStore, Host, Stopped } from "../src/volunteer/host";
import { parseExact, stringifyExact } from "../src/volunteer/json";
import { tar } from "../src/volunteer/tar";
import { confined, packPath, prepare } from "../src/volunteer/unit";

const engineDir = join(import.meta.dirname, "..", "..", "engine");
const wasmPath = join(engineDir, "target", "wasm32-wasip1", "release", "oikos.wasm");
const enc = new TextEncoder();

describe("tar", () => {
  it("writes an archive the system tar reads back", () => {
    const files = new Map<string, Uint8Array>([
      ["result.json", enc.encode('{"ok": true}\n')],
      ["out/activity.zarr/people/c/0/0", new Uint8Array(1500).map((_, i) => i % 251)],
      [`${"deep/".repeat(30)}leaf`, enc.encode("a long path splits into prefix and name")],
    ]);
    const dir = mkdtempSync(join(tmpdir(), "oikos-tar-"));
    writeFileSync(join(dir, "a.tar"), tar(files));
    execFileSync("tar", ["-xf", "a.tar"], { cwd: dir });
    for (const [path, bytes] of files) expect(new Uint8Array(readFileSync(join(dir, path)))).toEqual(bytes);
    expect(readdirSync(dir).sort()).toEqual(["a.tar", "deep", "out", "result.json"]);
  });

  it("stores paths of any length, as the native host's archives do", () => {
    const files = new Map([[`checkpoint-${"x".repeat(120)}.ckpt`, enc.encode("state")], [`${"y".repeat(150)}/${"z".repeat(110)}`, enc.encode("deep")]]);
    const dir = mkdtempSync(join(tmpdir(), "oikos-tar-"));
    writeFileSync(join(dir, "a.tar"), tar(files));
    const listed = execFileSync("python3", ["-I", "-c", "import sys, tarfile; print('\\n'.join(tarfile.open(sys.argv[1]).getnames()))", join(dir, "a.tar")], { encoding: "utf8" });
    expect(listed.trim().split("\n").sort()).toEqual([...files.keys()].sort());
  });

  it("refuses paths it cannot store or that leave the archive", () => {
    expect(() => tar(new Map([[`..\0${"x".repeat(100)}/escape`, new Uint8Array()]]))).toThrow(/inside/);
    expect(() => tar(new Map([["../x", new Uint8Array()]]))).toThrow(/inside/);
    expect(() => tar(new Map([["/x", new Uint8Array()]]))).toThrow(/inside/);
  });
});

describe("work units as JSON", () => {
  it("keep every number exactly as the coordinator wrote it", () => {
    const text = '{"seed": 9007199254740993, "params": {"a": 0.1, "b": 1e-7, "c": 2.50, "d": -12}, "years": 9000, "s": "1e5 \\" 9007199254740993"}';
    const unit = parseExact(text) as { years: number; params: { d: number }; s: string };
    expect(unit.years).toBe(9000);
    expect(unit.params.d).toBe(-12);
    expect(unit.s).toBe('1e5 " 9007199254740993');
    const back = stringifyExact(unit);
    for (const token of ["9007199254740993", "0.1", "1e-7", "2.50", "-12", "9000"]) expect(back).toContain(`: ${token}`.replace(": ", ":"));
    expect(JSON.parse(back).s).toBe(unit.s);
  });
});

describe("work units", () => {
  const unit = () => ({
    start: { kind: "initial" },
    activity: { dir: "out", every_years: 10, zones: { id: "f".repeat(64), path: "/elsewhere/zones.json" } },
    inputs: [{ id: "a".repeat(64), path: "/host/a.oikospack" }],
  });

  it("run inside the run directory, from their initial state", () => {
    expect(() => confined(unit())).not.toThrow();
    expect(() => confined({ ...unit(), checkpoint_out: "../escape" })).toThrow(/relative path inside/);
    expect(() => confined({ ...unit(), outputs: { dir: "/abs" } })).toThrow(/relative path inside/);
    expect(() => confined({ ...unit(), start: { kind: "checkpoint" } })).toThrow(/initial start/);
  });

  it("read their packs and zone file by id from the host's own copies", () => {
    const u = unit();
    const ids = prepare(u);
    expect(ids).toEqual(["a".repeat(64), "f".repeat(64)]);
    expect(u.inputs[0].path).toBe(packPath("a".repeat(64)));
    expect(u.activity.zones.path).toBe(packPath("f".repeat(64)));
    expect(() => prepare({ ...unit(), inputs: ["/etc/passwd"] })).toThrow(/\{id, path\} objects/);
    expect(() => prepare({ ...unit(), inputs: [{ id: "../x", path: "p" }] })).toThrow(/content hash/);
  });
});

// The browser runs the engine the gate checks: the wasm CLI, stripped of its debug
// sections as the build ships it, under the same WASI shim the page uses. It must
// reproduce the golden result exactly.
describe.skipIf(!existsSync(wasmPath))("the engine in the browser's WASI", () => {
  it("reproduces the golden work unit, reporting progress", async () => {
    const wasm = strip(readFileSync(wasmPath));
    expect(wasm.length).toBeLessThan(3_000_000);
    const module = await WebAssembly.compile(wasm);
    const unit = JSON.parse(readFileSync(join(engineDir, "golden", "m0-toy.workunit.json"), "utf8"));
    const golden = JSON.parse(readFileSync(join(engineDir, "golden", "m0-toy.json"), "utf8"));
    const years: number[] = [];
    const run = await runUnit(module, unit, new Map(), (done) => years.push(done));
    expect(run.result.state_hash).toBe(golden.state_hash);
    expect(run.result.summaries).toEqual(golden.summaries);
    expect(years.at(-1)).toBe(unit.years);
    expect([...run.files.keys()].sort()).toEqual(["result.json", "unit.json"]);
  }, 120_000);

  it("reports the engine's error when a unit cannot run", async () => {
    const module = await WebAssembly.compile(readFileSync(wasmPath));
    await expect(runUnit(module, { format: 99 }, new Map())).rejects.toThrow(/oikos/);
  }, 60_000);
});

/** A coordinator in memory: one unit, a pack, and what the host sent back. */
function coordinator(pack: Uint8Array, unitPack = pack) {
  const id = bytesToHex(blake3(unitPack));
  const sent: { method: string; path: string; body: unknown }[] = [];
  let leased = false;
  const fetch = async (url: string | URL | Request, init?: RequestInit): Promise<Response> => {
    const path = String(url).replace("http://coord", "");
    const method = init?.method ?? "GET";
    const body = init?.body instanceof Uint8Array ? init.body : init?.body ? JSON.parse(String(init.body)) : null;
    sent.push({ method, path, body });
    if (path === "/lease") {
      const units = leased ? [] : [{ lease: 7, unit_id: "u1", deadline: 0, unit: { start: { kind: "initial" }, years: 100, inputs: [{ id, path: "x" }] } }];
      leased = true;
      return Response.json(units);
    }
    if (path === `/packs/${id}`) return new Response(pack as BodyInit);
    return Response.json({});
  };
  return { fetch: fetch as typeof globalThis.fetch, sent, id };
}

class MemoryPacks implements PackStore {
  stored = new Map<string, Uint8Array>();
  async get(id: string) {
    return this.stored.get(id);
  }
  async put(id: string, bytes: Uint8Array) {
    this.stored.set(id, bytes);
  }
}

describe("the browser host", () => {
  const engine: Engine = async (unit, packs, onProgress) => {
    onProgress?.(100);
    const result = { state_hash: "h", unit_paths: (unit.inputs as { path: string }[]).map((i) => i.path), packs: [...packs.keys()] };
    return { result, files: new Map([["result.json", enc.encode(JSON.stringify(result))]]) };
  };

  it("leases a unit, fetches its pack once, runs it and returns the archive and result", async () => {
    const pack = enc.encode("pack bytes");
    const c = coordinator(pack);
    const packs = new MemoryPacks();
    const host = new Host("http://coord", "browser-test", engine, packs, c.fetch);
    const done = await host.round();
    expect(done?.unitId).toBe("u1");
    expect(packs.stored.get(c.id)).toEqual(pack);
    const calls = c.sent.map((s) => `${s.method} ${s.path}`);
    expect(calls).toEqual(["POST /lease", `GET /packs/${c.id}`, "PUT /archives/7?host=browser-test", "POST /report/7"]);
    const report = c.sent[3].body as { host: string; result: { unit_paths: string[]; packs: string[] } };
    expect(report.host).toBe("browser-test");
    expect(report.result.unit_paths).toEqual([packPath(c.id)]);
    expect(report.result.packs).toEqual([c.id]);
    expect(await host.round()).toBeNull();
  });

  it("refuses a pack whose bytes are not its id, and says so to the coordinator", async () => {
    const c = coordinator(enc.encode("tampered"), enc.encode("original"));
    const host = new Host("http://coord", "browser-test", engine, new MemoryPacks(), c.fetch);
    await expect(host.round()).rejects.toThrow(/does not match its id/);
    const fail = c.sent.at(-1)!;
    expect(fail.path).toBe("/fail/7");
    expect(String((fail.body as { error: string }).error)).toMatch(/does not match its id/);
  });

  it("leaves a unit the volunteer stopped to its lease, rather than report it failed", async () => {
    const c = coordinator(enc.encode("pack"));
    const stopped: Engine = async () => {
      throw new Stopped();
    };
    const host = new Host("http://coord", "browser-test", stopped, new MemoryPacks(), c.fetch);
    await expect(host.round()).rejects.toBeInstanceOf(Stopped);
    expect(c.sent.map((s) => s.path)).not.toContain("/fail/7");
  });

  it("stops a round mid-download without reporting the unit failed", async () => {
    const c = coordinator(enc.encode("pack"));
    const stop = new AbortController();
    const fetchFn = (async (url: string | URL | Request, init?: RequestInit) => {
      if (String(url).includes("/packs/")) {
        stop.abort();
        throw new DOMException("aborted", "AbortError");
      }
      return c.fetch(url, init);
    }) as typeof fetch;
    const host = new Host("http://coord", "browser-test", engine, new MemoryPacks(), fetchFn);
    await expect(host.round(undefined, stop.signal)).rejects.toBeInstanceOf(Stopped);
    expect(c.sent.map((s) => s.path)).not.toContain("/fail/7");
  });

  it("reports a run it cannot archive as failed, rather than strand it", async () => {
    const c = coordinator(enc.encode("pack"));
    const odd: Engine = async () => ({ result: { state_hash: "h" }, files: new Map([["../out", new Uint8Array()]]) });
    const host = new Host("http://coord", "browser-test", odd, new MemoryPacks(), c.fetch);
    await expect(host.round()).rejects.toThrow(/inside/);
    expect(c.sent.at(-1)!.path).toBe("/fail/7");
  });
});
