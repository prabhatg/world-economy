/**
 * A minimal Zarr v3 reader for what the engine writes (engine/crates/oikos-io/src/zarr.rs):
 * uint64 arrays, `bytes` (little-endian) then `gzip` codecs, a regular chunk grid and
 * "/"-separated chunk keys. Chunks are fetched on demand and cached, so the viewer
 * reads only the times and cells it shows. Browsers decompress gzip natively
 * (DecompressionStream), so no codec library is needed.
 */

export type Fetcher = (url: string) => Promise<Response>;

export interface ArrayMeta {
  shape: number[];
  chunks: number[];
  attributes: Record<string, unknown>;
}

/** Checks a `zarr.json` is an array this reader can decode; errors name `path`. */
export function parseMeta(json: any, path: string): ArrayMeta {
  if (json?.zarr_format !== 3 || json?.node_type !== "array") throw new Error(`${path}: not a Zarr v3 array`);
  if (json.data_type !== "uint64") throw new Error(`${path}: data type ${json.data_type}, expected uint64`);
  const codecs = (json.codecs ?? []).map((c: any) => c.name).join(", ");
  const endian = json.codecs?.[0]?.configuration?.endian;
  if (codecs !== "bytes, gzip" || endian !== "little") {
    throw new Error(`${path}: codecs ${codecs}, expected bytes (little-endian), gzip`);
  }
  if (json.chunk_key_encoding?.configuration?.separator !== "/") throw new Error(`${path}: chunk keys must use "/"`);
  return { shape: json.shape, chunks: json.chunk_grid.configuration.chunk_shape, attributes: json.attributes ?? {} };
}

async function get(fetcher: Fetcher, url: string): Promise<Response> {
  const r = await fetcher(url);
  if (!r.ok) throw new Error(`${url}: HTTP ${r.status}`);
  return r;
}

async function gunzip(bytes: ArrayBuffer): Promise<ArrayBuffer> {
  const stream = new Blob([bytes]).stream().pipeThrough(new DecompressionStream("gzip"));
  return new Response(stream).arrayBuffer();
}

/** Chunks kept per array; a world's field at 91 times in M1 is 30 chunks. */
const CACHE = 64;

export class ZarrArray {
  readonly shape: number[];
  readonly attributes: Record<string, unknown>;
  private readonly chunks: number[];
  private readonly cache = new Map<string, Promise<BigUint64Array>>();

  private constructor(
    private readonly url: string,
    meta: ArrayMeta,
    private readonly fetcher: Fetcher,
  ) {
    this.shape = meta.shape;
    this.chunks = meta.chunks;
    this.attributes = meta.attributes;
  }

  static async open(url: string, fetcher: Fetcher = (u) => fetch(u)): Promise<ZarrArray> {
    const path = `${url}/zarr.json`;
    return new ZarrArray(url, parseMeta(await (await get(fetcher, path)).json(), path), fetcher);
  }

  private chunk(key: string): Promise<BigUint64Array> {
    let c = this.cache.get(key);
    if (!c) {
      c = get(this.fetcher, `${this.url}/c/${key}`)
        .then((r) => r.arrayBuffer())
        .then(gunzip)
        .then((raw) => {
          const view = new DataView(raw);
          const out = new BigUint64Array(raw.byteLength / 8);
          for (let i = 0; i < out.length; i++) out[i] = view.getBigUint64(i * 8, true);
          return out;
        });
      c.catch(() => this.cache.delete(key));
      this.cache.set(key, c);
      if (this.cache.size > CACHE) this.cache.delete(this.cache.keys().next().value!);
    }
    return c;
  }

  /** Row `t` of a 2-D (time, cell) array, every cell, as numbers. */
  async row(t: number): Promise<Float64Array> {
    const [times, cells] = this.shape;
    const [tc, cc] = this.chunks;
    if (this.shape.length !== 2 || t < 0 || t >= times) throw new Error(`${this.url}: no row ${t}`);
    const ti = Math.floor(t / tc);
    const r = t % tc;
    const parts = await Promise.all(
      Array.from({ length: Math.ceil(cells / cc) }, (_, ci) => this.chunk(`${ti}/${ci}`)),
    );
    const out = new Float64Array(cells);
    parts.forEach((chunk, ci) => {
      const width = Math.min(cc, cells - ci * cc);
      for (let j = 0; j < width; j++) out[ci * cc + j] = Number(chunk[r * cc + j]);
    });
    return out;
  }

  /** A whole 1-D array, exactly (H3 indexes need all 64 bits). */
  async bigints(): Promise<BigUint64Array> {
    const [n] = this.shape;
    const [c] = this.chunks;
    if (this.shape.length !== 1) throw new Error(`${this.url}: not 1-D`);
    const parts = await Promise.all(Array.from({ length: Math.ceil(n / c) }, (_, i) => this.chunk(`${i}`)));
    const out = new BigUint64Array(n);
    parts.forEach((chunk, i) => out.set(chunk.subarray(0, Math.min(c, n - i * c)), i * c));
    return out;
  }
}
