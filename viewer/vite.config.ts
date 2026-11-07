import { createReadStream, existsSync, readFileSync, statSync } from "node:fs";
import { join, normalize, sep } from "node:path";
import type { Connect, Plugin } from "vite";
import { defineConfig } from "vitest/config";
import { ENGINE, strip } from "./scripts/engine.mjs";

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

// The volunteer page (#22) in development: the engine as the build ships it, and the
// coordinator's routes passed to one running locally (oikos-coord serve), so the page
// talks to it as it does when the coordinator serves the build.
const engine: Plugin = {
  name: "manyworlds-engine",
  configureServer: (server) =>
    void server.middlewares.use("/oikos.wasm", (_req, res, next) => {
      if (!existsSync(ENGINE)) return next();
      res.setHeader("Content-Type", "application/wasm");
      res.end(strip(readFileSync(ENGINE)));
    }),
};
const coordinator = process.env.OIKOS_COORDINATOR ?? "http://127.0.0.1:8650";
const routes = ["/lease", "/packs", "/archives", "/report", "/fail", "/units", "/status"];

// The design tokens live in ../design (one source for every surface), so the dev
// server may read the parent directory. `base: "./"` lets the build run from any path.
export default defineConfig({
  base: "./",
  publicDir: false,
  plugins: [ensembles, engine],
  server: { fs: { allow: [".."] }, proxy: Object.fromEntries(routes.map((r) => [r, coordinator])) },
  build: { rolldownOptions: { input: { viewer: "index.html", volunteer: "volunteer.html" } } },
  test: { environment: "node", include: ["test/**/*.test.ts"] },
});
