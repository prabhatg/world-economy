import { gzipSync } from "node:zlib";
import { describe, expect, it } from "vitest";
import { ZarrArray, parseMeta } from "../src/zarr";

/** A uint64 chunk encoded as the engine writes it: little-endian bytes, then gzip. */
function chunk(values: (number | bigint)[]): Uint8Array {
  const view = new DataView(new ArrayBuffer(values.length * 8));
  values.forEach((v, i) => view.setBigUint64(i * 8, BigInt(v), true));
  return gzipSync(new Uint8Array(view.buffer));
}

function meta(shape: number[], chunks: number[], attributes = {}) {
  return {
    zarr_format: 3,
    node_type: "array",
    shape,
    data_type: "uint64",
    chunk_grid: { name: "regular", configuration: { chunk_shape: chunks } },
    chunk_key_encoding: { name: "default", configuration: { separator: "/" } },
    fill_value: 0,
    codecs: [
      { name: "bytes", configuration: { endian: "little" } },
      { name: "gzip", configuration: { level: 5 } },
    ],
    attributes,
    dimension_names: shape.length === 2 ? ["time", "cell"] : ["cell"],
  };
}

/** A fetcher over an in-memory store that counts requests. */
function store(files: Record<string, object | Uint8Array>) {
  const seen: string[] = [];
  const fetcher = async (url: string) => {
    seen.push(url);
    const f = files[url];
    if (f === undefined) return new Response(null, { status: 404 });
    return new Response(f instanceof Uint8Array ? new Blob([new Uint8Array(f)]) : JSON.stringify(f));
  };
  return { fetcher, seen };
}

describe("zarr", () => {
  // 3 times x 5 cells in chunks of 2 times x 3 cells; edge chunks are padded.
  const files = {
    "s/a/zarr.json": meta([3, 5], [2, 3], { scale_factor: 1e-4 }),
    "s/a/c/0/0": chunk([1, 2, 3, 6, 7, 8]),
    "s/a/c/0/1": chunk([4, 5, 0, 9, 10, 0]),
    "s/a/c/1/0": chunk([11, 12, 13, 0, 0, 0]),
    "s/a/c/1/1": chunk([14, 15, 0, 0, 0, 0]),
    "s/h/zarr.json": meta([2], [4]),
    "s/h/c/0": chunk([0x85283473fffffffn, 2n ** 53n - 1n, 0, 0]),
  };

  it("reads rows across cell chunks and clips the padding", async () => {
    const { fetcher } = store(files);
    const a = await ZarrArray.open("s/a", fetcher);
    expect(a.shape).toEqual([3, 5]);
    expect(a.attributes.scale_factor).toBe(1e-4);
    expect(Array.from(await a.row(0))).toEqual([1, 2, 3, 4, 5]);
    expect(Array.from(await a.row(1))).toEqual([6, 7, 8, 9, 10]);
    expect(Array.from(await a.row(2))).toEqual([11, 12, 13, 14, 15]);
  });

  it("fetches each chunk once", async () => {
    const { fetcher, seen } = store(files);
    const a = await ZarrArray.open("s/a", fetcher);
    await Promise.all([a.row(0), a.row(1), a.row(0)]);
    expect(seen.filter((u) => u.includes("/c/")).sort()).toEqual(["s/a/c/0/0", "s/a/c/0/1"]);
  });

  it("reads 1-D arrays exactly as 64-bit integers", async () => {
    const { fetcher } = store(files);
    const h = await ZarrArray.open("s/h", fetcher);
    expect(Array.from(await h.bigints())).toEqual([0x85283473fffffffn, 2n ** 53n - 1n]);
  });

  it("rejects what the reader cannot decode, saying what it found", () => {
    expect(() => parseMeta({ ...meta([2], [2]), data_type: "float32" }, "x")).toThrow(/x: data type float32/);
    expect(() => parseMeta({ ...meta([2], [2]), codecs: [{ name: "zstd" }] }, "x")).toThrow(/x: codecs zstd/);
  });

  it("reports a missing array with its path", async () => {
    const { fetcher } = store(files);
    await expect(ZarrArray.open("s/missing", fetcher)).rejects.toThrow(/s\/missing\/zarr.json: HTTP 404/);
  });
});
