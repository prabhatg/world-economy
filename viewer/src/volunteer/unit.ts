// The checks a host makes on a leased work unit before running it, as the headless
// host does (oikos-coord `confined` and `files_mut`): it must not trust its coordinator.

type Json = Record<string, unknown>;

/** A pack's place in the engine's file system: the host's own copy, by id. */
export function packPath(id: string): string {
  return `/packs/${id}.oikospack`;
}

/** Throws unless `unit` starts from its initial state and writes only inside the run directory. */
export function confined(unit: Json): void {
  if ((unit.start as Json | undefined)?.kind !== "initial") {
    throw new Error("v0 runs only work units with an initial start");
  }
  const paths: [string, unknown][] = [
    ["checkpoint_out", unit.checkpoint_out],
    ["outputs.dir", (unit.outputs as Json | undefined)?.dir],
    ["activity.dir", (unit.activity as Json | undefined)?.dir],
  ];
  for (const [field, path] of paths) {
    if (path === undefined || path === null) continue;
    const inside =
      typeof path === "string" && path !== "" && !path.startsWith("/") && !path.split("/").includes("..");
    if (!inside) throw new Error(`${field} ${JSON.stringify(path)} must be a relative path inside the run directory`);
  }
}

/** Points `unit`'s packs and zone file at the host's copies; returns their ids, in order. */
export function prepare(unit: Json): string[] {
  const refs: unknown[] = [];
  if (Array.isArray(unit.inputs)) refs.push(...unit.inputs);
  const zones = (unit.activity as Json | undefined)?.zones;
  if (zones !== undefined && zones !== null) refs.push(zones);
  if (refs.some((r) => typeof r !== "object" || r === null || Array.isArray(r))) {
    throw new Error("file references (inputs, activity.zones) must be {id, path} objects");
  }
  return (refs as Json[]).map((r) => {
    const id = r.id;
    if (typeof id !== "string" || !/^[0-9a-f]{64}$/.test(id)) {
      throw new Error(`pack id ${JSON.stringify(id)} is not a content hash`);
    }
    r.path = packPath(id);
    return id;
  });
}
