import assert from "node:assert/strict";
import test from "node:test";
import { deviceOrientationToHorizontal, syntheticDeviceOrientation } from "../src/orientation.mjs";

const close = (actual, expected) => assert.ok(
  Math.abs(actual - expected) < 1e-8, `${actual} != ${expected}`,
);
const headingClose = (actual, expected) => close(
  ((actual - expected + 540) % 360) - 180, 0,
);

test("raw portrait sensor pitch sweeps through upright without yawing", () => {
  for (const heading of [0, 45, 90, 123, 180, 270, 359]) {
    for (let beta = 5; beta <= 175; beta += 5) {
      const result = deviceOrientationToHorizontal({
        alpha: (360 - heading) % 360, beta, gamma: 0, absolute: true,
      }, heading);
      headingClose(result.heading, heading);
      close(result.altitude, beta - 90);
    }
  }
});

test("raw yaw sweeps leave pitch unchanged", () => {
  for (const beta of [15, 60, 90, 120, 165]) {
    for (let alpha = 0; alpha < 360; alpha += 5) {
      const result = deviceOrientationToHorizontal({ alpha, beta, gamma: 0, absolute: true }, 0);
      headingClose(result.heading, (360 - alpha) % 360);
      close(result.altitude, beta - 90);
    }
  }
});

test("sideways phones use gamma for pitch, not alpha or screen angle", () => {
  for (const heading of [0, 90, 180, 270]) {
    for (const pitch of [-80, -30, 0, 30, 80]) {
      const result = deviceOrientationToHorizontal({
        alpha: ((pitch > 0 ? 270 : 90) - heading + 360) % 360,
        beta: pitch > 0 ? -180 : 0,
        gamma: pitch > 0 ? 90 - pitch : -90 - pitch,
        absolute: true,
      }, heading);
      headingClose(result.heading, heading);
      close(result.altitude, pitch);
    }
  }
});

test("synthetic events reconstruct independent physical basis vectors at any screen roll", () => {
  const r = Math.PI / 180;
  for (const roll of [0, 30, 90, 150, 180, 270]) {
    for (const heading of [0, 45, 90, 123, 180, 270, 359]) {
      for (const pitch of [-85, -45, -0.001, 0, 0.001, 45, 85]) {
        const event = syntheticDeviceOrientation(heading, pitch, roll);
        assert.ok(event.alpha >= 0 && event.alpha < 360);
        assert.ok(event.beta >= -180 && event.beta < 180);
        assert.ok(event.gamma >= -90 - 1e-8 && event.gamma <= 90 + 1e-8);
        const a = event.alpha * r, b = event.beta * r, g = event.gamma * r;
        // Full W3C matrix; check all three axes, not just the conversion's output.
        const matrix = [
          [Math.cos(a) * Math.cos(g) - Math.sin(a) * Math.sin(b) * Math.sin(g),
            -Math.cos(b) * Math.sin(a), Math.cos(g) * Math.sin(a) * Math.sin(b) + Math.cos(a) * Math.sin(g)],
          [Math.cos(g) * Math.sin(a) + Math.cos(a) * Math.sin(b) * Math.sin(g),
            Math.cos(a) * Math.cos(b), Math.sin(a) * Math.sin(g) - Math.cos(a) * Math.cos(g) * Math.sin(b)],
          [-Math.cos(b) * Math.sin(g), Math.sin(b), Math.cos(b) * Math.cos(g)],
        ];
        const h = heading * r, p = pitch * r, s = roll * r;
        const right = [Math.cos(h), -Math.sin(h), 0];
        const up = [-Math.sin(h) * Math.sin(p), -Math.cos(h) * Math.sin(p), Math.cos(p)];
        const forward = [Math.sin(h) * Math.cos(p), Math.cos(h) * Math.cos(p), Math.sin(p)];
        for (let i = 0; i < 3; i++) {
          close(matrix[i][0], right[i] * Math.cos(s) + up[i] * Math.sin(s));
          close(matrix[i][1], up[i] * Math.cos(s) - right[i] * Math.sin(s));
          close(matrix[i][2], -forward[i]);
        }
        const result = deviceOrientationToHorizontal(event, heading);
        headingClose(result.heading, heading);
        close(result.altitude, pitch);
      }
    }
  }
});

test("zenith and nadir preserve yaw without clamping elevation to 85 degrees", () => {
  for (const beta of [0, 180]) {
    const result = deviceOrientationToHorizontal({ alpha: 234, beta, gamma: 0, absolute: true }, 123);
    close(result.heading, 123);
    close(result.altitude, beta === 0 ? -90 : 90);
  }
});

test("invalid sensor data is rejected, including missing gamma", () => {
  for (const gamma of [null, undefined, NaN, Infinity]) {
    assert.throws(() => deviceOrientationToHorizontal({ alpha: 0, beta: 90, gamma }, 0), TypeError);
  }
});

test("Safari compass calibrates the relative frame and retains it near vertical", () => {
  const calibrated = deviceOrientationToHorizontal({
    alpha: 30, beta: 60, gamma: 0, absolute: false,
    webkitCompassHeading: 100, webkitCompassAccuracy: 5,
  }, 0);
  headingClose(calibrated.heading, 100);
  close(calibrated.altitude, -30);
  assert.equal(calibrated.absolute, true);
  const upright = deviceOrientationToHorizontal({
    alpha: 20, beta: 90, gamma: 0, absolute: false,
    webkitCompassHeading: 270, webkitCompassAccuracy: 5,
  }, calibrated.heading, calibrated.compassOffset);
  headingClose(upright.heading, 110);
  close(upright.altitude, 0);
  const invalidCompass = deviceOrientationToHorizontal({
    alpha: 30, beta: 60, gamma: 0, absolute: false,
    webkitCompassHeading: 100, webkitCompassAccuracy: -1,
  }, 0);
  assert.equal(invalidCompass.absolute, false);
  headingClose(invalidCompass.heading, 330);
  const absolute = deviceOrientationToHorizontal({
    alpha: 30, beta: 60, gamma: 0, absolute: true,
  }, 0, calibrated.compassOffset);
  headingClose(absolute.heading, 330);
});
