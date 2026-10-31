// Renders the viewer with a real ensemble (viewer/data/m1) at phone and desktop widths
// and fails on page errors, an empty map, sideways scroll or a year that never draws.
// Skips, with a notice, when no ensemble has been built (it is not committed).
import { existsSync } from "node:fs";
import { chromium } from "playwright";
import { createServer } from "vite";

const root = new URL("..", import.meta.url).pathname;
if (!existsSync(`${root}data/m1/catalog.json`)) {
  console.log("SKIP viewer smoke: no ensemble in viewer/data/m1 (python -m oikos_cal ensemble ...)");
  process.exit(0);
}

if (!existsSync(chromium.executablePath())) {
  console.log("SKIP viewer smoke: no Playwright Chromium (npx playwright install chromium)");
  process.exit(0);
}

const server = await createServer({ root, server: { port: 0, strictPort: false }, logLevel: "error" });
await server.listen();
const url = server.resolvedUrls.local[0];
// Headless Chromium draws WebGL in software.
const browser = await chromium.launch({ args: ["--use-gl=angle", "--use-angle=swiftshader", "--enable-unsafe-swiftshader"] });
const problems = [];
try {
  for (const [name, width, height] of [["phone", 390, 844], ["desktop", 1440, 900]]) {
    const page = await browser.newPage({ viewport: { width, height } });
    page.on("pageerror", (e) => problems.push(`${name}: ${e.message}`));
    await page.goto(url, { waitUntil: "networkidle" });
    await page.waitForFunction(() => document.getElementById("year")?.textContent?.includes("BCE"), null, { timeout: 30_000 });
    await page.click("#play");
    await page.waitForTimeout(1500);
    await page.click("#play");
    const check = await page.evaluate(() => {
      const canvas = document.querySelector("#map canvas");
      return {
        overflow: document.documentElement.scrollWidth > innerWidth,
        year: document.getElementById("year").textContent,
        status: document.getElementById("status").textContent,
        canvas: canvas ? canvas.width * canvas.height : 0,
      };
    });
    // A drawn M1 map lights about 60% of its area; a blank one only its overlays
    // (3% on desktop, 21% on a phone, where the legend covers more of it).
    const shot = (await page.locator("#map").screenshot()).toString("base64");
    const drawn = await page.evaluate(async (b64) => {
      const img = new Image();
      img.src = `data:image/png;base64,${b64}`;
      await img.decode();
      const c = Object.assign(document.createElement("canvas"), { width: img.width, height: img.height });
      const g = c.getContext("2d");
      g.drawImage(img, 0, 0);
      const d = g.getImageData(0, 0, c.width, c.height).data;
      const night = getComputedStyle(document.documentElement).getPropertyValue("--night").trim();
      const n = [1, 3, 5].map((i) => parseInt(night.slice(i, i + 2), 16));
      let lit = 0, all = 0;
      for (let i = 0; i < d.length; i += 4 * 31, all++) {
        if (Math.abs(d[i] - n[0]) + Math.abs(d[i + 1] - n[1]) + Math.abs(d[i + 2] - n[2]) > 24) lit++;
      }
      return lit / all;
    }, shot);
    if (check.overflow) problems.push(`${name}: the page scrolls sideways`);
    if (check.status) problems.push(`${name}: ${check.status}`);
    if (!check.canvas || drawn < 0.4) problems.push(`${name}: the map did not draw (${Math.round(drawn * 100)}% lit)`);
    console.log(`viewer smoke ${name}: ${check.year}, ${Math.round(drawn * 100)}% of the map drawn`);
    await page.close();
  }
} finally {
  await browser.close();
  await server.close();
}
if (problems.length) {
  console.error(problems.join("\n"));
  process.exit(1);
}
console.log("viewer smoke ok");
