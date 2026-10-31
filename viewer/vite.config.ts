import { createReadStream, existsSync, statSync } from "node:fs";
import { join, normalize, sep } from "node:path";
import type { Connect, Plugin } from "vite";
import { defineConfig } from "vitest/config";

// Ensembles are large and built locally (viewer/data, not committed), so they are not
// in publicDir (a build would copy them); the dev and preview servers serve them at
// /data. A hosted build gets an ensemble placed next to index.html as data/.
const dataDir = join(import.meta.dirname, "data");

const serveData: Connect.NextHandleFunction = (req, res, next) => {
  const path = normalize(join(dataDir, decodeURIComponent((req.url ?? "").split("?")[0])));
  if (!path.startsWith(dataDir + sep) || !existsSync(path) || !statSync(path).isFile()) return next();
  res.setHeader("Content-Type", path.endsWith(".json") ? "application/json" : "application/octet-stream");
  createReadStream(path).pipe(res);
};

const ensembles: Plugin = {
  name: "manyworlds-ensembles",
  configureServer: (server) => void server.middlewares.use("/data", serveData),
  configurePreviewServer: (server) => void server.middlewares.use("/data", serveData),
};

// The design tokens live in ../design (one source for every surface), so the dev
// server may read the parent directory. `base: "./"` lets the build run from any path.
export default defineConfig({
  base: "./",
  publicDir: false,
  plugins: [ensembles],
  server: { fs: { allow: [".."] } },
  test: { environment: "node", include: ["test/**/*.test.ts"] },
});
