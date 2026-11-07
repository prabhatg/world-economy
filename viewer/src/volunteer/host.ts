// The browser host (#22): the protocol of the headless one (oikos-coord work) over
// fetch. Lease a unit, check it, fetch its packs by id (each checked against its
// content hash, and kept for the next unit), run it, upload the run directory as a
// tar archive and report the engine's result; or report why it failed.

import { blake3 } from "@noble/hashes/blake3.js";
import { bytesToHex } from "@noble/hashes/utils.js";
import type { Run } from "./engine";
import { parseExact, stringifyExact } from "./json";
import { tar } from "./tar";
import { confined, prepare } from "./unit";

/** Runs a prepared unit with its packs (id → bytes). */
export type Engine = (
  unit: Record<string, unknown>,
  packs: Map<string, Uint8Array>,
  onProgress?: (years: number) => void,
) => Promise<Run>;

/** Where a host keeps packs between units. */
export interface PackStore {
  get(id: string): Promise<Uint8Array | undefined>;
  put(id: string, bytes: Uint8Array): Promise<void>;
}

export type Stage =
  | { stage: "packs"; done: number; of: number }
  | { stage: "run"; years: number; of: number; firstYear: number }
  | { stage: "upload" };

/** The volunteer stopped the run. Not a failure of the unit: its lease runs out and
 * another host gets it, so stopping never counts towards giving a unit up. */
export class Stopped extends Error {
  constructor() {
    super("stopped by the volunteer");
  }
}

export interface Done {
  unitId: string;
  run: Run;
  years: number;
}

interface Lease {
  lease: number;
  unit_id: string;
  unit: Record<string, unknown>;
}

function checked(id: string, bytes: Uint8Array): Uint8Array {
  if (bytesToHex(blake3(bytes)) !== id) throw new Error(`pack ${id}: content does not match its id`);
  return bytes;
}

export class Host {
  constructor(
    readonly base: string,
    readonly host: string,
    private readonly engine: Engine,
    private readonly packs: PackStore,
    private readonly fetchFn: typeof fetch = (...a) => fetch(...a),
  ) {}

  private signal?: AbortSignal;

  private async call(path: string, init?: RequestInit): Promise<Response> {
    const r = await this.fetchFn(`${this.base}${path}`, { ...init, signal: this.signal });
    if (!r.ok) throw new Error(`${init?.method ?? "GET"} ${path}: HTTP ${r.status} ${await r.text()}`);
    return r;
  }

  private post(path: string, body: unknown): Promise<Response> {
    return this.call(path, { method: "POST", headers: { "Content-Type": "application/json" }, body: stringifyExact(body) });
  }

  /** The pack's bytes: the stored copy, or fetched and stored; checked either way. */
  private async pack(id: string): Promise<Uint8Array> {
    const kept = await this.packs.get(id);
    if (kept) return checked(id, kept);
    const bytes = checked(id, new Uint8Array(await (await this.call(`/packs/${id}`)).arrayBuffer()));
    await this.packs.put(id, bytes);
    return bytes;
  }

  /** Leases, runs and reports one unit; null when the coordinator has none. Aborting
   * `signal` (the volunteer's Stop) ends the round with `Stopped`, leaving the unit to
   * its lease. Units are parsed losslessly (`parseExact`), so the engine runs them as sent. */
  async round(onStage?: (s: Stage) => void, signal?: AbortSignal): Promise<Done | null> {
    this.signal = signal;
    const stopped = () => {
      if (signal?.aborted) throw new Stopped();
    };
    const leases = parseExact(await (await this.post("/lease", { host: this.host, n: 1 })).text()) as Lease[];
    const l = leases[0];
    if (!l) return null;
    let run: Run;
    let archive: Uint8Array;
    try {
      const unit = l.unit;
      confined(unit);
      const ids = prepare(unit);
      const packs = new Map<string, Uint8Array>();
      for (const [i, id] of ids.entries()) {
        onStage?.({ stage: "packs", done: i, of: ids.length });
        packs.set(id, await this.pack(id));
      }
      const of = Number(unit.years ?? 0);
      const firstYear = -12_000; // an initial start is the engine's epoch (oikos-core EPOCH_YEAR)
      onStage?.({ stage: "run", years: 0, of, firstYear });
      stopped();
      run = await this.engine(unit, packs, (years) => onStage?.({ stage: "run", years, of, firstYear }));
      archive = tar(run.files);
    } catch (e) {
      if (signal?.aborted) throw new Stopped();
      if (!(e instanceof Stopped)) {
        const error = e instanceof Error ? e.message : String(e);
        await this.post(`/fail/${l.lease}`, { host: this.host, error }).catch(() => {});
      }
      throw e;
    }
    stopped();
    onStage?.({ stage: "upload" });
    await this.call(`/archives/${l.lease}?host=${encodeURIComponent(this.host)}`, {
      method: "PUT",
      headers: { "Content-Type": "application/x-tar" },
      body: archive as BodyInit,
    });
    await this.post(`/report/${l.lease}`, { host: this.host, result: run.result });
    return { unitId: l.unit_id, run, years: Number(l.unit.years ?? 0) };
  }
}
