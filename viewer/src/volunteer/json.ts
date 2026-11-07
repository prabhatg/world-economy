// Work units and results as JSON, losslessly. JavaScript numbers are doubles: a seed
// above 2^53 would change on parsing, and a parameter written `1e-7` or `2.50` would
// come back re-spelled for the engine's own parser. Every host must run the unit the coordinator
// sent, to the bit, so any number JavaScript would not write back exactly as given
// travels as a marked string, and is written back as its original text.

const MARK = "\u0000number:";

/** The number tokens of `text` outside strings, with their offsets. */
function* numbers(text: string): Generator<[number, number]> {
  for (let i = 0; i < text.length; ) {
    const c = text[i];
    if (c === '"') {
      for (i++; text[i] !== '"'; i++) if (text[i] === "\\") i++;
      i++;
    } else if (c === "-" || (c >= "0" && c <= "9")) {
      const start = i;
      while (i < text.length && /[-+.eE0-9]/.test(text[i])) i++;
      yield [start, i];
    } else i++;
  }
}

/** Parses JSON (a work unit, a result), keeping numbers JavaScript cannot round-trip as their text. */
export function parseExact(text: string): unknown {
  let out = "";
  let at = 0;
  for (const [start, end] of numbers(text)) {
    const token = text.slice(start, end);
    if (String(Number(token)) === token) continue;
    out += text.slice(at, start) + JSON.stringify(MARK + token);
    at = end;
  }
  return JSON.parse(out + text.slice(at));
}

/** Writes JSON parsed by `parseExact`, its kept numbers as they were given. */
export function stringifyExact(value: unknown): string {
  const escaped = JSON.stringify(MARK).slice(1, -1);
  return JSON.stringify(value).replace(new RegExp(`"${escaped.replace(/\\/g, "\\\\")}([-+.eE0-9]+)"`, "g"), "$1");
}
