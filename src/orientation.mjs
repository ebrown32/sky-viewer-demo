const radians = Math.PI / 180;
const degrees = 180 / Math.PI;
const normalize = (angle) => ((angle % 360) + 360) % 360;

export function deviceOrientationToHorizontal(event, previousHeading, compassOffset = null) {
  if (![event.alpha, event.beta, event.gamma].every(Number.isFinite)) {
    throw new TypeError("Device orientation requires finite alpha, beta, and gamma");
  }
  const alpha = event.alpha * radians;
  const beta = event.beta * radians;
  const gamma = event.gamma * radians;
  const ca = Math.cos(alpha);
  const sa = Math.sin(alpha);
  const cb = Math.cos(beta);
  const sb = Math.sin(beta);
  const cg = Math.cos(gamma);
  const sg = Math.sin(gamma);

  // Rz(alpha) Rx(beta) Ry(gamma) applied to the rear-camera direction (0, 0, -1).
  const east = -ca * sg - sa * sb * cg;
  const north = -sa * sg + ca * sb * cg;
  const up = -cb * cg;
  const horizontalLength = Math.hypot(east, north);

  // Safari supplies magnetic heading for the top edge, while alpha is relative.
  // Calibrate that frame only when the top edge has a usable horizontal projection.
  if (!event.absolute && Number.isFinite(event.webkitCompassHeading) &&
      event.webkitCompassHeading >= 0 && event.webkitCompassHeading < 360 &&
      Number.isFinite(event.webkitCompassAccuracy) && event.webkitCompassAccuracy >= 0 &&
      Math.abs(cb) > 0.2) {
    const topHeading = Math.atan2(-sa * cb, ca * cb) * degrees;
    compassOffset = normalize(event.webkitCompassHeading - topHeading);
  }
  const absolute = Boolean(event.absolute) || compassOffset !== null;
  const offset = event.absolute ? 0 : (compassOffset ?? 0);
  return {
    // Azimuth is undefined at the zenith/nadir; retain yaw instead of amplifying noise.
    heading: horizontalLength > 1e-6
      ? normalize(Math.atan2(east, north) * degrees + offset)
      : previousHeading,
    altitude: Math.atan2(up, horizontalLength) * degrees,
    absolute,
    compassOffset,
  };
}

export function syntheticDeviceOrientation(heading, pitch, screenAngle = 0) {
  if (![heading, pitch, screenAngle].every(Number.isFinite)) {
    throw new TypeError("Synthetic orientation requires finite heading, pitch, and screen angle");
  }
  const h = heading * radians;
  const p = pitch * radians;
  const roll = screenAngle * radians;
  const right = [Math.cos(h), -Math.sin(h), 0];
  const up = [-Math.sin(h) * Math.sin(p), -Math.cos(h) * Math.sin(p), Math.cos(p)];
  const back = [-Math.sin(h) * Math.cos(p), -Math.cos(h) * Math.cos(p), -Math.sin(p)];
  const x = right.map((value, i) => value * Math.cos(roll) + up[i] * Math.sin(roll));
  const y = up.map((value, i) => value * Math.cos(roll) - right[i] * Math.sin(roll));
  let alpha;
  let beta = Math.asin(Math.max(-1, Math.min(1, y[2])));
  let gamma;
  if (Math.abs(Math.cos(beta)) < 1e-7) {
    alpha = Math.atan2(x[1], x[0]);
    gamma = 0;
  } else {
    alpha = Math.atan2(-y[0], y[1]);
    gamma = Math.atan2(-x[2], back[2]);
    if (Math.abs(gamma) > Math.PI / 2) {
      alpha += Math.PI;
      beta = beta >= 0 ? Math.PI - beta : -Math.PI - beta;
      gamma += gamma > 0 ? -Math.PI : Math.PI;
    }
  }
  return {
    alpha: normalize(alpha * degrees),
    beta: ((beta * degrees + 180) % 360 + 360) % 360 - 180,
    gamma: gamma * degrees,
    absolute: true,
  };
}
