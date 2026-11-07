// A run directory as a POSIX ustar archive, as the headless host uploads it
// (oikos-coord work): the coordinator keeps it per unit and host, and calibration
// reads the record out of it. Files only, in path order; parent directories are
// implied by the paths. A path that does not fit ustar's fields goes in a PAX header.

const enc = new TextEncoder();

/** Writes `text` as ASCII into `block` at `at`, at most `len` bytes. */
function put(block: Uint8Array, at: number, len: number, text: string): void {
  block.set(enc.encode(text).subarray(0, len), at);
}

/** An octal field of `len` bytes: digits, zero-padded, then a NUL. */
function octal(block: Uint8Array, at: number, len: number, n: number): void {
  put(block, at, len, n.toString(8).padStart(len - 1, "0") + "\0");
}

/** Splits `path` into ustar's prefix (≤155 bytes) and name (≤100), at a slash; null if it cannot. */
function split(path: string): [string, string] | null {
  if (enc.encode(path).length <= 100) return ["", path];
  for (let i = path.indexOf("/"); i >= 0; i = path.indexOf("/", i + 1)) {
    const [prefix, name] = [path.slice(0, i), path.slice(i + 1)];
    if (enc.encode(prefix).length <= 155 && enc.encode(name).length <= 100) return [prefix, name];
  }
  return null;
}

/** A PAX record `"<len> path=<path>\n"`, its length counting itself. */
function paxPath(path: string): Uint8Array {
  const body = ` path=${path}\n`;
  const n = enc.encode(body).length;
  let len = n + String(n).length;
  if (String(len).length !== String(n).length) len = n + String(len).length;
  return enc.encode(`${len}${body}`);
}

function header(path: string, size: number, type = 0x30): Uint8Array {
  const block = new Uint8Array(512);
  const [prefix, name] = split(path) ?? ["", path.slice(-100)];
  put(block, 0, 100, name);
  octal(block, 100, 8, 0o644);
  octal(block, 108, 8, 0);
  octal(block, 116, 8, 0);
  octal(block, 124, 12, size);
  octal(block, 136, 12, 0);
  block.fill(0x20, 148, 156); // the checksum counts its own field as spaces
  block[156] = type; // 0x30 a regular file, 0x78 ('x') a PAX header for the next one
  put(block, 257, 8, "ustar\u000000");
  put(block, 345, 155, prefix);
  const sum = block.reduce((a, b) => a + b, 0);
  put(block, 148, 8, sum.toString(8).padStart(6, "0") + "\0 ");
  return block;
}

/** The archive of `files` (relative path → bytes). */
export function tar(files: Map<string, Uint8Array>): Uint8Array {
  const paths = [...files.keys()].sort();
  for (const p of paths) {
    if (!p || p.includes("\0") || p.startsWith("/") || p.split("/").some((c) => c === ".." || c === "")) {
      throw new Error(`${JSON.stringify(p)}: an archive keeps paths inside the run directory`);
    }
  }
  const pad = (n: number) => Math.ceil(n / 512) * 512;
  const blocks: Uint8Array[] = [];
  for (const p of paths) {
    const bytes = files.get(p)!;
    if (!split(p)) {
      const pax = paxPath(p);
      blocks.push(header("PaxHeader", pax.length, 0x78), pax, new Uint8Array(pad(pax.length) - pax.length));
    }
    blocks.push(header(p, bytes.length), bytes, new Uint8Array(pad(bytes.length) - bytes.length));
  }
  blocks.push(new Uint8Array(1024)); // two zero blocks end the archive
  const out = new Uint8Array(blocks.reduce((n, b) => n + b.length, 0));
  blocks.reduce((at, b) => (out.set(b, at), at + b.length), 0);
  return out;
}
