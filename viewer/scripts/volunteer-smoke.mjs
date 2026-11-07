// The browser client end to end (#22): a coordinator serving the built page, headless
// Chromium pressing Start, and the headless native host as the second computer. Both
// units must reach quorum: the browser's results equal native ones bit for bit. Also
// fails on page errors or sideways scroll at phone and desktop widths. Skips, with a
// notice, without the built page and engine, the coordinator binary or Chromium.
//   SHOTS=dir node scripts/volunteer-smoke.mjs    (also saves screenshots)
import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { chromium } from "playwright";

const root = join(import.meta.dirname, "..");
const engine = join(root, "..", "engine");
const coord = join(engine, "target", "release", "oikos-coord");
const oikos = join(engine, "target", "release", "oikos");
const skip = (why) => {
  console.log(`SKIP volunteer smoke: ${why}`);
  process.exit(0);
};
if (!existsSync(join(root, "dist", "oikos.wasm"))) skip("no dist/oikos.wasm (npm run build after the wasm engine)");
if (!existsSync(coord) || !existsSync(oikos)) skip("no release oikos-coord and oikos (cargo build --release)");
if (!existsSync(chromium.executablePath())) skip("no Playwright Chromium (npx playwright install chromium)");

const dir = mkdtempSync(join(tmpdir(), "oikos-volunteer-"));
const server = spawn(coord, ["serve", "--dir", join(dir, "coord"), "--addr", "127.0.0.1:0", "--client", join(root, "dist")]);
const base = await new Promise((resolve, reject) => {
  let err = "";
  server.stderr.on("data", (d) => {
    err += d;
    const m = /on (http:\/\/[\d.:]+)/.exec(err);
    if (m) resolve(m[1]);
  });
  server.on("exit", (code) => reject(new Error(`oikos-coord serve exited ${code}: ${err}`)));
});
const units = ["golden/m0-toy.workunit.json", "examples/outputs.workunit.json"].map((p, i) => ({
  id: ["golden", "outputs"][i],
  unit: JSON.parse(readFileSync(join(engine, p), "utf8")),
}));
const json = async (path, body) =>
  (await fetch(`${base}${path}`, body && { method: "POST", headers: { "Content-Type": "application/json" }, body: JSON.stringify(body) })).json();
await json("/experiments/smoke", units);

const browser = await chromium.launch();
const problems = [];
const shots = process.env.SHOTS;
if (shots) mkdirSync(shots, { recursive: true });
try {
  const page = await browser.newPage({ viewport: { width: 390, height: 844 } });
  page.on("pageerror", (e) => problems.push(`page error: ${e.message}`));
  await page.goto(`${base}/client/volunteer.html`, { waitUntil: "networkidle" });
  if (shots) await page.screenshot({ path: join(shots, "volunteer-phone-idle.png"), fullPage: true });
  await page.click("#toggle");
  await page.waitForFunction(() => document.getElementById("worlds")?.textContent === "2", null, { timeout: 180_000 });
  const phone = await page.evaluate(() => ({
    overflow: document.documentElement.scrollWidth > innerWidth,
    verdict: document.getElementById("verdict").textContent,
    headline: document.getElementById("now-headline").textContent,
  }));
  if (phone.overflow) problems.push("phone: the page scrolls sideways");
  if (!/second computer/.test(phone.verdict)) problems.push(`phone: verdict ${JSON.stringify(phone.verdict)}`);
  if (shots) {
    await page.emulateMedia({ colorScheme: "dark" });
    await page.screenshot({ path: join(shots, "volunteer-phone-dark.png"), fullPage: true });
  }
  await page.click("#toggle");
  console.log(`volunteer smoke: the browser computed 2 worlds (${phone.headline})`);

  const native = spawnSync(coord, ["work", "--coordinator", base, "--host", "native", "--oikos", oikos,
    "--cache", join(dir, "cache"), "--scratch", join(dir, "runs"), "--until-idle"], { encoding: "utf8" });
  if (native.status !== 0) problems.push(`native host: ${native.stderr}`);
  for (const { id } of units) {
    const u = await json(`/units/${id}`);
    if (u.status !== "done") problems.push(`${id}: ${u.status}, not validated by the two hosts`);
  }
  const golden = JSON.parse(readFileSync(join(engine, "golden", "m0-toy.json"), "utf8"));
  const accepted = (await json("/units/golden")).report?.state_hash;
  if (accepted !== golden.state_hash) problems.push(`golden: accepted ${accepted}, not the golden hash`);

  const desk = await browser.newPage({ viewport: { width: 1440, height: 900 } });
  desk.on("pageerror", (e) => problems.push(`page error: ${e.message}`));
  await desk.goto(`${base}/client/volunteer.html`, { waitUntil: "networkidle" });
  if (await desk.evaluate(() => document.documentElement.scrollWidth > innerWidth)) problems.push("desktop: sideways scroll");
  if (shots) await desk.screenshot({ path: join(shots, "volunteer-desktop-idle.png"), fullPage: true });
} finally {
  await browser.close();
  server.kill();
}
if (problems.length) {
  console.error(problems.join("\n"));
  process.exit(1);
}
console.log("volunteer smoke ok: both units validated, the browser's results equal the native host's");
