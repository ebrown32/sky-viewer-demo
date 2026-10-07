import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { resolve, join, extname } from "node:path";
import { syntheticDeviceOrientation } from "../src/orientation.mjs";

const site = process.env.SKY_TEST_URL;
const dist = resolve("dist");
const server = site ? null : createServer(async (request, response) => {
  try {
    const path = new URL(request.url, "http://localhost").pathname.replace(/^\/sky-viewer-demo/, "");
    const file = resolve(dist, `.${path.endsWith("/") ? `${path}index.html` : path}`);
    if (!file.startsWith(`${dist}/`)) throw new Error("Invalid asset path");
    const body = await readFile(file);
    response.setHeader("Content-Type", {
      ".html": "text/html", ".js": "text/javascript", ".mjs": "text/javascript",
      ".wasm": "application/wasm", ".css": "text/css",
    }[extname(file)] ?? "application/octet-stream");
    response.end(body);
  } catch (error) {
    response.writeHead(404);
    response.end(error.message);
  }
});
if (server) await new Promise((done) => server.listen(0, "127.0.0.1", done));
const base = site ?? `http://127.0.0.1:${server.address().port}/sky-viewer-demo/`;
const profile = await mkdtemp(join(tmpdir(), "sky-orientation-"));
const chrome = spawn(process.env.CHROME_BIN ?? "chromium", [
  "--headless", "--no-sandbox", "--disable-dev-shm-usage", "--enable-unsafe-swiftshader",
  "--use-gl=angle", "--use-angle=swiftshader", "--remote-debugging-port=0",
  `--user-data-dir=${profile}`, "--no-first-run", "about:blank",
], { stdio: ["ignore", "ignore", "pipe"] });
let socket;
try {
  const browserUrl = await new Promise((done, reject) => {
    const timeout = setTimeout(() => reject(new Error("Chrome startup timed out")), 30000);
    let output = "";
    chrome.on("error", reject);
    chrome.on("exit", (code) => reject(new Error(`Chrome exited: ${code}\n${output}`)));
    chrome.stderr.on("data", (data) => {
      output += data;
      const match = output.match(/DevTools listening on (ws:\/\/[^\s]+)/);
      if (match) {
        clearTimeout(timeout);
        done(match[1]);
      }
    });
  });
  const endpoint = new URL(browserUrl);
  const targets = await (await fetch(`http://${endpoint.host}/json/list`)).json();
  socket = new WebSocket(targets.find((target) => target.type === "page").webSocketDebuggerUrl);
  await new Promise((done, reject) => {
    socket.addEventListener("open", done, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });
  let id = 0;
  const pending = new Map();
  const errors = [];
  socket.addEventListener("message", ({ data }) => {
    const message = JSON.parse(data);
    if (message.method === "Runtime.exceptionThrown") {
      const details = message.params.exceptionDetails;
      // Winit's WASM event-loop startup deliberately throws this exact sentinel.
      if (!details.exception?.description?.startsWith(
        "Error: Using exceptions for control flow, don't mind me. This isn't actually an error!\n",
      )) errors.push(details);
    }
    if (message.id) {
      const request = pending.get(message.id);
      pending.delete(message.id);
      if (message.error) request.reject(new Error(JSON.stringify(message.error)));
      else request.done(message.result);
    }
  });
  const send = (method, params = {}) => new Promise((done, reject) => {
    const requestId = ++id;
    pending.set(requestId, { done, reject });
    socket.send(JSON.stringify({ id: requestId, method, params }));
  });
  const evaluate = async (expression) => {
    const result = await send("Runtime.evaluate", { expression, returnByValue: true, awaitPromise: true });
    if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
    return result.result.value;
  };
  await send("Runtime.enable");
  await send("Page.enable");
  await send("Emulation.setDeviceMetricsOverride", {
    width: 390, height: 844, deviceScaleFactor: 1, mobile: true,
  });
  // Choose a fixed time when the Moon is above Seattle's horizon.
  await send("Page.addScriptToEvaluateOnNewDocument", {
    source: "Date.now = () => 1791478800000;",
  });
  await send("Page.navigate", { url: `${base}?sensor-test` });
  const waitFor = async (expression) => {
    const deadline = Date.now() + 90000;
    while (Date.now() < deadline) {
      if (await evaluate(expression)) return;
      await new Promise((done) => setTimeout(done, 100));
    }
    throw new Error(`Timed out: ${expression}`);
  };
  await waitFor("window.skyState?.skyReady && document.getElementById('startup-screen').hidden");
  const settle = async () => {
    await evaluate("new Promise(done => { let frames = 0; const next = () => ++frames >= 5 ? done() : requestAnimationFrame(next); requestAnimationFrame(next); })");
  };
  const apply = async (event, type = "deviceorientation") => {
    await evaluate(`(() => {
      const event = new Event(${JSON.stringify(type)});
      for (const [key, value] of Object.entries(${JSON.stringify(event)})) {
        Object.defineProperty(event, key, { value });
      }
      window.dispatchEvent(event);
    })()`);
    await settle();
    return evaluate("({ heading: skyState.heading, altitude: skyState.altitude, ra: skyState.rightAscension, dec: skyState.declination })");
  };
  const close = (actual, expected, tolerance = 0.002) => assert.ok(
    Math.abs(actual - expected) < tolerance, `${actual} != ${expected}`,
  );
  const headingClose = (actual, expected) => close(((actual - expected + 540) % 360) - 180, 0);
  let cases = 0;
  for (const heading of [0, 90, 123, 180, 270, 359]) {
    for (const beta of [5, 30, 60, 89, 90, 91, 120, 150, 175]) {
      const state = await apply({ alpha: (360 - heading) % 360, beta, gamma: 0, absolute: true });
      headingClose(state.heading, heading);
      close(state.altitude, beta - 90);
      cases++;
    }
  }
  for (const roll of [90, 180, 270]) {
    for (const heading of [0, 90, 180, 270]) {
      for (const pitch of [-60, 0, 60]) {
        const state = await apply(syntheticDeviceOrientation(heading, pitch, roll));
        headingClose(state.heading, heading);
        close(state.altitude, pitch);
        cases++;
      }
    }
  }
  await apply({ alpha: 270, beta: 120, gamma: 0, absolute: true }, "deviceorientationabsolute");
  const afterRelative = await apply({ alpha: 180, beta: 60, gamma: 0, absolute: false });
  headingClose(afterRelative.heading, 90);
  close(afterRelative.altitude, 30);
  const beforeInvalid = afterRelative;
  const afterInvalid = await apply({ alpha: 0, beta: 0, gamma: null, absolute: true });
  headingClose(afterInvalid.heading, beforeInvalid.heading);
  close(afterInvalid.altitude, beforeInvalid.altitude);
  assert.equal(await evaluate("document.getElementById('sensor-status').textContent"), "COMPASS DATA UNAVAILABLE");

  // Aim the physical rear-camera ray at actual catalog/body directions, then inspect rendered labels.
  const moon = await evaluate("skyState.objects.find(object => object[0] === 'Moon')");
  assert.ok(moon && moon[2] > 0, `Moon must be above the horizon: ${JSON.stringify(moon)}`);
  const polaris = await evaluate("skyState.objects.find(object => object[0] === 'Polaris')");
  assert.ok(polaris && polaris[2] > 46 && polaris[2] < 49);
  for (const object of [moon, polaris]) {
    const state = await apply(syntheticDeviceOrientation(object[1], object[2], 90));
    headingClose(state.heading, object[1]);
    close(state.altitude, object[2]);
    const label = await evaluate(`skyState.projectedLabels.find(label => label[0] === ${JSON.stringify(object[0])})`);
    assert.ok(label, `${object[0]} not rendered`);
    close(label[1], 195, 1);
    close(label[2], 422, 1);
  }
  const poleState = await apply(syntheticDeviceOrientation(0, 47.6062));
  close(poleState.dec, 90, 0.01);

  // A fixed object moves down, not sideways, when pitching up; east is screen-right.
  await apply(syntheticDeviceOrientation(moon[1], moon[2]));
  const centered = await evaluate("skyState.projectedLabels.find(label => label[0] === 'Moon')");
  await apply(syntheticDeviceOrientation(moon[1], moon[2] + 5));
  const pitched = await evaluate("skyState.projectedLabels.find(label => label[0] === 'Moon')");
  close(pitched[1], centered[1], 0.1);
  assert.ok(pitched[2] > centered[2] + 10, "Pitch must move the Moon vertically downward");
  await apply(syntheticDeviceOrientation(moon[1] - 5, moon[2]));
  const yawed = await evaluate("skyState.projectedLabels.find(label => label[0] === 'Moon')");
  assert.ok(yawed[1] > centered[1] + 10, "East must appear on the right");
  assert.deepEqual(errors, [], "Unexpected browser exceptions");
  console.log(`Headless Chrome: ${cases} sensor poses passed; Moon/Polaris centered; pitch vertical; yaw unmirrored; absolute event preferred.`);
} finally {
  socket?.close();
  chrome.kill();
  await new Promise((done) => chrome.exitCode !== null ? done() : chrome.once("exit", done));
  if (server) await new Promise((done) => server.close(done));
  await rm(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 100 });
}
