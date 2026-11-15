// SPDX-License-Identifier: MPL-2.0
// CrySirror showcase: a brand intro, then an FPV drone flight through a neon
// city whose screens show live native games. The recorder host
// (`crysirror showcase`) boots each game headless and publishes its picture
// as `__surface/<slot>`; this script only lays out the world, binds those
// surfaces as material textures and flies the camera. The brief comes from
// the host as save file `showcase.json`.

interface ScreenBrief { slot: number; caption: string; engine: string; size: [number, number]; surface: string; }
interface Brief { fps: number; seconds: number; intro_seconds: number; width: number; height: number; screens: ScreenBrief[]; }

const brief: Brief = JSON.parse(fs.readSave("showcase.json") ?? "{}");
const W = brief.width, H = brief.height, U = H / 2160;
const INTRO = brief.intro_seconds, END = brief.seconds;
const OUTRO = 9.0;

type V = [number, number, number];
const add = (a: V, b: V): V => [a[0] + b[0], a[1] + b[1], a[2] + b[2]];
const sub = (a: V, b: V): V => [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
const mul = (a: V, s: number): V => [a[0] * s, a[1] * s, a[2] * s];
const dot = (a: V, b: V) => a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
const cross = (a: V, b: V): V => [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
const len = (a: V) => Math.hypot(a[0], a[1], a[2]);
const norm = (a: V): V => { const l = len(a) || 1; return [a[0] / l, a[1] / l, a[2] / l]; };
const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
const lerpV = (a: V, b: V, t: number): V => [lerp(a[0], b[0], t), lerp(a[1], b[1], t), lerp(a[2], b[2], t)];
const clamp = (x: number, a: number, b: number) => Math.max(a, Math.min(b, x));
const smooth = (x: number) => { const t = clamp(x, 0, 1); return t * t * (3 - 2 * t); };
let seed = 7;
const rand = () => { seed = (seed * 1103515245 + 12345) & 0x7fffffff; return seed / 0x7fffffff; };

// ---------------------------------------------------------------- world ----
const PALETTE: [number, number, number][] = [
  [0.1, 0.9, 1.0], [1.0, 0.25, 0.7], [0.55, 0.35, 1.0], [0.2, 1.0, 0.6], [1.0, 0.6, 0.15],
];
function glow(c: [number, number, number], strength: number): Record<string, any> {
  return { base_color_factor: [0.02, 0.02, 0.025, 1], emissive_factor: c, emissive_strength: strength, roughness_factor: 0.4 };
}
function box(name: string, position: V, scale: V, material: Record<string, any>, rotation: V = [0, 0, 0]): Entity {
  return world.spawn({ name, position, rotation, scale, geometry: "cube", material });
}

interface Screen { slot: number; center: V; normal: V; right: V; h: number; w: number; brief: ScreenBrief; color: [number, number, number]; view: V; }
const screens: Screen[] = [];
const gates: { center: V; dir: V }[] = [];

function buildCity(): void {
  world.setEnvironment({
    intensity: 0.06, exposure: 0.3, tonemap: "aces", show_sky: false,
    bloom: 0.09, ambient_occlusion: 0.8, temporal_aa: true, shadow_distance: 120,
    sun: { direction: [0.3, -0.8, 0.45], color: [0.55, 0.65, 1.0], intensity: 0.6, shadows: true },
    fog: { density: 0.006, height_falloff: 0.05, color: [0.015, 0.02, 0.045] },
    reflections: { max_distance: 80, steps: 48, thickness: 0.6, intensity: 1.0 },
  });
  // Glossy floor with an emissive grid.
  box("floor", [0, -0.1, -150], [700, 0.2, 700], { base_color_factor: [0.012, 0.014, 0.02, 1], roughness_factor: 0.12, metallic_factor: 0.6 });
  for (let i = -30; i <= 30; i++) {
    const c: [number, number, number] = i % 5 === 0 ? [0.2, 0.7, 1.0] : [0.08, 0.2, 0.4];
    box(`gx${i}`, [i * 10, 0.01, -150], [0.06, 0.02, 700], glow(c, i % 5 === 0 ? 3 : 1.5));
    box(`gz${i}`, [0, 0.01, -150 + i * 10], [700, 0.02, 0.06], glow(c, i % 5 === 0 ? 3 : 1.5));
  }
  const n = brief.screens.length;
  const spacing = 34;
  for (let i = 0; i < n; i++) {
    const b = brief.screens[i];
    const side = i % 2 === 0 ? -1 : 1;
    const h = 7 + (i % 3) * 2.2;
    const w = h * b.size[0] / b.size[1];
    const center: V = [side * (10 + (i % 3) * 2), h / 2 + 3 + (i % 4) * 1.5, -i * spacing - 20];
    const normal = norm([-side * 0.8, 0, 0.6]);
    const yaw = Math.atan2(normal[0], normal[2]) * 180 / Math.PI;
    const right = norm(cross([0, 1, 0], normal));
    const color = PALETTE[i % PALETTE.length];
    const screen = world.spawn({
      name: `screen:${b.slot}`, position: center, rotation: [0, yaw, 0], scale: [w, h, 1],
      geometry: { kind: "model", asset: "screen.glb", mesh: 0 },
      material: { base_color_texture: b.surface, emissive_texture: b.surface, base_color_factor: [0, 0, 0, 1],
        emissive_factor: [1, 1, 1], emissive_strength: 0.92, roughness_factor: 0.85, specular_factor: 0.2, double_sided: true },
    });
    // Bezel, back plate, pylon and accent light.
    const back = sub(center, mul(normal, 0.18));
    box(`bezel:${i}`, back, [w + 0.5, h + 0.5, 0.3], { base_color_factor: [0.03, 0.03, 0.04, 1], roughness_factor: 0.35, metallic_factor: 0.9 }, [0, yaw, 0]);
    box(`edge-top:${i}`, add(center, [0, h / 2 + 0.3, 0]), [w + 0.6, 0.08, 0.08], glow(color, 9), [0, yaw, 0]);
    box(`edge-bottom:${i}`, add(center, [0, -h / 2 - 0.3, 0]), [w + 0.6, 0.08, 0.08], glow(color, 9), [0, yaw, 0]);
    box(`pylon:${i}`, [back[0] - normal[0] * 0.6, (center[1] - h / 2) / 2, back[2] - normal[2] * 0.6], [0.6, center[1] - h / 2, 0.6], { base_color_factor: [0.04, 0.04, 0.05, 1], metallic_factor: 0.8, roughness_factor: 0.3 });
    world.spawn({ name: `light:${i}`, position: add(center, add(mul(normal, 2), [0, h / 2 + 2.5, 0])), light: { shape: { kind: "point" }, color, intensity: 500, range: 18 } });
    const d = h / 0.95;
    screens.push({ slot: b.slot, center, normal, right, h, w, brief: b, color, view: add(center, mul(normal, d)) });
    void screen;
  }
  // Race gates between consecutive screens, on the flight line.
  for (let i = 0; i + 1 < screens.length; i++) {
    const a = screens[i].view, b = screens[i + 1].view;
    const c: V = [lerp(a[0], b[0], 0.5) * 0.3, 2.6 + (i % 2) * 1.2, lerp(a[2], b[2], 0.5)];
    const dir = norm(sub(b, a));
    const yaw = Math.atan2(dir[0], dir[2]) * 180 / Math.PI;
    const col = PALETTE[(i + 2) % PALETTE.length];
    const r = 2.2;
    for (let k = 0; k < 16; k++) {
      const ang = k / 16 * Math.PI * 2;
      const rr = norm(cross([0, 1, 0], dir));
      const p = add(c, add(mul(rr, Math.cos(ang) * r), [0, Math.sin(ang) * r, 0]));
      box(`gate:${i}:${k}`, p, [0.32, 0.32, 0.32], glow(col, 12), [0, yaw, ang * 180 / Math.PI]);
    }
    world.spawn({ name: `gate-light:${i}`, position: c, light: { shape: { kind: "point" }, color: col, intensity: 400, range: 12 } });
    gates.push({ center: c, dir });
  }
  // Skyline: towers with emissive window strips.
  const depth = screens.length * spacing + 60;
  for (let i = 0; i < 90; i++) {
    const side = i % 2 === 0 ? -1 : 1;
    const x = side * (32 + rand() * 90);
    const z = 20 - rand() * (depth + 80);
    const h = 18 + rand() * rand() * 110;
    const wdt = 5 + rand() * 9;
    box(`tower:${i}`, [x, h / 2, z], [wdt, h, wdt], { base_color_factor: [0.02, 0.022, 0.03, 1], roughness_factor: 0.3, metallic_factor: 0.7 });
    const col = PALETTE[Math.floor(rand() * PALETTE.length)];
    const strips = 1 + Math.floor(rand() * 4);
    for (let s = 0; s < strips; s++) {
      const y = 4 + rand() * (h - 6);
      box(`strip:${i}:${s}`, [x, y, z], [wdt + 0.08, 0.25, wdt + 0.08], glow(col, 5 + rand() * 5));
    }
  }
  // Floating data motes along the route.
  for (let i = 0; i < screens.length; i += 2) {
    world.spawn({ name: `motes:${i}`, position: [0, 6, screens[i].center[2]], particles: { rate: 30, lifetime: [4, 7], speed: [0.2, 0.6], spread_degrees: 180,
      gravity: [0, 0.15, 0], drag: 0.1, size: [0.06, 0.02], color_start: [2, 8, 12, 1], color_end: [4, 1, 8, 0], max: 220, seed: i + 1 } });
  }
}

// ---------------------------------------------------------------- intro ----
const LOGO: V = [0, 40, 420];
let logo: Entity, sweep: Entity, sparks: Entity | null = null;
function buildIntro(): void {
  logo = world.spawn({ name: "logo", position: LOGO, rotation: [0, 180, 0], scale: [6, 6, 1], geometry: { kind: "model", asset: "screen.glb", mesh: 0 },
    material: { base_color_texture: "logo.png", emissive_texture: "logo.png", base_color_factor: [0.1, 0.1, 0.1, 1], emissive_factor: [0, 0, 0],
      emissive_strength: 1, alpha_mode: "blend", unlit: false, double_sided: true } });
  sweep = world.spawn({ name: "sweep", position: add(LOGO, [-9, 0, -0.4]), rotation: [0, 0, -28], scale: [0.35, 14, 0.02], geometry: "cube",
    material: { base_color_factor: [0, 0, 0, 1], emissive_factor: [0.6, 1.0, 1.0], emissive_strength: 0, alpha_mode: "additive" } });
}
function setLogoGlow(level: number): void {
  logo.setMaterial({ base_color_texture: "logo.png", emissive_texture: "logo.png", base_color_factor: [0.08 * Math.min(1, level), 0.08 * Math.min(1, level), 0.08 * Math.min(1, level), clamp(level, 0, 1)],
    emissive_factor: [level, level, level], emissive_strength: 1.25, alpha_mode: "blend", double_sided: true });
}

// --------------------------------------------------------------- layers ----
const FONT = "default";
function label(id: number, s: string, x: number, y: number, size: number, color: number[], opacity = 255, width = 3000): void {
  layers.put({ id, rect: [x * U, y * U, width * U, size * 1.6 * U], paint: { kind: "text", text: s, font: FONT, size: Math.round(size * U), color }, opacity: Math.round(clamp(opacity, 0, 255)) });
}
function solid(id: number, x: number, y: number, w: number, h: number, color: number[], opacity = 255): void {
  layers.put({ id, rect: [x * U, y * U, w * U, h * U], paint: { kind: "solid", color }, opacity: Math.round(clamp(opacity, 0, 255)) });
}
const widths = new Map<string, number>();
function textWidth(s: string, size: number): number {
  const px = Math.round(size * U), key = `${px}:${s}`;
  let w = widths.get(key);
  if (w === undefined) {
    w = Number(text.measureSimple(FONT, px, s).advance) / U;
    widths.set(key, w);
  }
  return w;
}
function centered(id: number, s: string, y: number, size: number, color: number[], opacity: number): void {
  label(id, s, (3840 - textWidth(s, size)) / 2, y, size, color, opacity);
}

// --------------------------------------------------------------- flight ----
interface Key { t: number; eye: V; look: V; focus: number; }
const keys: Key[] = [];
function buildFlight(): void {
  const start = INTRO, flightEnd = END - OUTRO;
  const per = (flightEnd - start) / Math.max(1, screens.length);
  const first = screens[0];
  keys.push({ t: start - 0.6, eye: [first.view[0] * 0.2, 26, 40], look: [0, 4, -30], focus: 0 });
  keys.push({ t: start + 0.6, eye: [first.view[0] * 0.4, 14, 18], look: first.center, focus: 0.2 });
  screens.forEach((s, i) => {
    const t0 = start + i * per + 1.0;
    const approach = add(s.view, add(mul(s.right, 3.2 * (i % 2 ? 1 : -1)), [0, 1.2, 4]));
    keys.push({ t: t0, eye: approach, look: s.center, focus: 0.8 });
    keys.push({ t: t0 + per * 0.28, eye: add(s.view, mul(s.right, 0.6 * (i % 2 ? 1 : -1))), look: s.center, focus: 1 });
    keys.push({ t: t0 + per * 0.58, eye: add(s.view, add(mul(s.right, -0.8 * (i % 2 ? 1 : -1)), [0, -0.4, -1.2])), look: s.center, focus: 1 });
    if (i < gates.length) {
      const g = gates[i];
      keys.push({ t: t0 + per * 0.8, eye: sub(g.center, mul(g.dir, 6)), look: g.center, focus: 0 });
      keys.push({ t: t0 + per * 0.9, eye: g.center, look: add(g.center, g.dir), focus: 0 });
    }
  });
  // Outro: climb and pull back over the whole city.
  const last = screens[screens.length - 1];
  const mid: V = [0, 6, (screens[0].center[2] + last.center[2]) / 2];
  keys.push({ t: flightEnd + 1.4, eye: add(last.view, [0, 14, -14]), look: mid, focus: 0.3 });
  keys.push({ t: flightEnd + 3.2, eye: [0, 26, mid[2]], look: [0, 4, screens[0].center[2] + 10], focus: 0.4 });
  keys.push({ t: END - 4.5, eye: [0, 32, screens[0].center[2] + 45], look: mid, focus: 0.5 });
  keys.push({ t: END + 1, eye: [0, 40, screens[0].center[2] + 62], look: mid, focus: 0.5 });
}
function catmull(p0: V, p1: V, p2: V, p3: V, t: number): V {
  const t2 = t * t, t3 = t2 * t;
  const f = (a: number, b: number, c: number, d: number) => 0.5 * (2 * b + (-a + c) * t + (2 * a - 5 * b + 4 * c - d) * t2 + (-a + 3 * b - 3 * c + d) * t3);
  return [f(p0[0], p1[0], p2[0], p3[0]), f(p0[1], p1[1], p2[1], p3[1]), f(p0[2], p1[2], p2[2], p3[2])];
}
function sample(t: number): { eye: V; look: V; focus: number } {
  let i = 0;
  while (i + 1 < keys.length - 1 && keys[i + 1].t <= t) i++;
  const a = keys[Math.max(0, i - 1)], b = keys[i], c = keys[Math.min(keys.length - 1, i + 1)], d = keys[Math.min(keys.length - 1, i + 2)];
  const u = clamp((t - b.t) / Math.max(1e-3, c.t - b.t), 0, 1);
  const e = smooth(u) * 0.35 + u * 0.65;
  return { eye: catmull(a.eye, b.eye, c.eye, d.eye, e), look: catmull(a.look, b.look, c.look, d.look, e), focus: lerp(b.focus, c.focus, smooth(u)) };
}
let roll = 0, prevVel: V = [0, 0, -1], prevEye: V | null = null;
function flyCamera(t: number, dt: number): { eye: V; speed: number } {
  const s = sample(t), ahead = sample(t + 0.12);
  const vel = mul(sub(ahead.eye, s.eye), 1 / 0.12);
  const speed = len(vel);
  const travel = speed > 0.5 ? norm(vel) : norm(sub(s.look, s.eye));
  const toLook = norm(sub(s.look, s.eye));
  const fwd = norm(lerpV(travel, toLook, s.focus));
  const right = norm(cross(fwd, [0, 1, 0]));
  const accel = mul(sub(vel, prevVel), 1 / Math.max(dt, 1e-3));
  prevVel = vel;
  const lateral = clamp(dot(accel, right) / 9.81, -1.2, 1.2);
  const target = -Math.atan(lateral) * 0.9 * (1 - 0.75 * s.focus);
  roll += (target - roll) * clamp(dt * 3, 0, 1);
  // FPV airframe vibration.
  const shake = 0.012 + speed * 0.0008;
  const eye = add(s.eye, [Math.sin(t * 37.1) * shake, Math.sin(t * 29.3 + 1) * shake, 0]);
  const up0 = norm(cross(right, fwd));
  const up = norm(add(mul(up0, Math.cos(roll)), mul(right, Math.sin(roll))));
  const fov = 74 + clamp(speed * 0.35, 0, 14) - 14 * s.focus;
  world.setCamera({ eye, target: add(eye, fwd), up, fov, near: 0.05, far: 900 });
  prevEye = eye;
  return { eye, speed };
}

// ------------------------------------------------------------ lifecycle ----
let t = 0;
function onStart(): void {
  buildCity();
  buildIntro();
  buildFlight();
  engine.log(`showcase: ${screens.length} screens, ${keys.length} flight keys, ${W}x${H}`);
  solid(1, 0, 0, 3840, 2160, [255, 255, 255, 255], 0); // flash
  solid(2, 0, 0, 3840, 2160, [0, 0, 0, 255], 255);     // fade
}

function intro(t: number): void {
  const reveal = Math.max(1.5, INTRO * 0.42);
  const k = smooth((t - 0.4) / (reveal - 0.4));
  const after = Math.max(0, t - reveal);
  const glowLevel = k * 0.55 + (after > 0 ? 0.45 * Math.exp(-after * 1.2) + 0.25 * (1 - Math.exp(-after * 2)) : 0);
  setLogoGlow(Math.min(1.4, glowLevel + 0.15 * k));
  const s = 5.2 + 0.8 * smooth(t / INTRO);
  logo.scale = [s, s, 1];
  logo.rotation = [0, 180 + (1 - k) * 18, 0];
  // Light sweep across the logo after the reveal.
  const sw = clamp((t - reveal - 0.15) / 1.1, 0, 1);
  sweep.position = add(LOGO, [9 - 18 * sw, 0, -0.5]);
  sweep.setMaterial({ base_color_factor: [0, 0, 0, 1], emissive_factor: [0.6, 1.0, 1.0], emissive_strength: sw > 0 && sw < 1 ? 6 * Math.sin(sw * Math.PI) : 0, alpha_mode: "additive" });
  if (!sparks && t >= reveal) {
    sparks = world.spawn({ name: "sparks", position: add(LOGO, [0, 0, -0.6]), particles: { rate: 900, lifetime: [0.6, 1.8], speed: [2, 9], spread_degrees: 180,
      gravity: [0, -1.5, 0], drag: 1.2, size: [0.07, 0.01], color_start: [6, 22, 26, 1], color_end: [8, 2, 14, 0], max: 900, seed: 5 } });
  }
  if (sparks && t > reveal + 0.25) sparks.setParticles({ rate: 0, lifetime: [0.6, 1.8], speed: [2, 9], spread_degrees: 180,
      gravity: [0, -1.5, 0], drag: 1.2, size: [0.07, 0.01], color_start: [6, 22, 26, 1], color_end: [8, 2, 14, 0], max: 900, seed: 5 });
  // Push-in, then a fast dive through the logo at the end.
  const push = smooth((t - (INTRO - 1.3)) / 1.3);
  const z = LOGO[2] - 22 + 4 * smooth(t / INTRO) + 19 * push * push;
  world.setCamera({ eye: [0.4 * Math.sin(t * 0.7), LOGO[1] + 0.3, z], target: [0, LOGO[1], LOGO[2]], fov: 40 + 25 * push, near: 0.05, far: 900 });
  const title = smooth((t - reveal - 0.3) / 1.0) * (1 - smooth((t - (INTRO - 1.0)) / 0.5));
  centered(10, "C R Y S I R R O R", 1560, 150, [235, 250, 255, 255], title * 255);
  centered(11, "E N G I N E", 1780, 60, [120, 220, 235, 255], title * 210);
  layers.setOpacity(2, Math.round(255 * (1 - smooth(t / 0.6))));
  const flash = smooth((t - (INTRO - 0.15)) / 0.15);
  layers.setOpacity(1, Math.round(255 * flash * 0.75));
}

function hud(t: number, eye: V, speed: number): void {
  const k = smooth((t - INTRO - 0.4) / 0.8) * (1 - smooth((t - (END - OUTRO + 0.5)) / 1));
  const a = k * 230;
  const c = [210, 245, 255, 255];
  solid(20, 80, 80, 26, 26, [255, 40, 60, 255], (Math.floor(t * 2) % 2 ? 0.25 : 1) * a);
  label(21, "REC  CRYSIRROR LIVE", 130, 60, 48, c, a);
  label(22, `${brief.screens.length} NATIVE GAMES  ·  ONE RENDERER`, 80, 2000, 48, c, a * 0.9);
  label(23, `ALT ${eye[1].toFixed(1).padStart(5)} m`, 3240, 60, 48, c, a);
  label(24, `SPD ${(speed * 3.6).toFixed(0).padStart(4)} km/h`, 3240, 124, 48, c, a);
  // Caption of the screen the drone is looking at.
  let best = -1, score = 0;
  screens.forEach((s, i) => {
    const d = len(sub(s.center, eye));
    const f = dot(norm(sub(s.center, eye)), s.normal) < -0.3 ? 1 / (1 + d * d / 300) : 0;
    if (f > score) { score = f; best = i; }
  });
  const vis = best >= 0 ? smooth((score - 0.35) / 0.25) * k : 0;
  if (best >= 0) {
    const s = screens[best];
    solid(30, 0, 1740, 1500, 4, [Math.round(s.color[0] * 255), Math.round(s.color[1] * 255), Math.round(s.color[2] * 255), 255], vis * 255);
    label(31, s.brief.engine.toUpperCase(), 80, 1600, 88, [255, 255, 255, 255], vis * 255);
    label(32, s.brief.caption, 84, 1768, 56, [190, 225, 235, 255], vis * 230);
  }
}

function outro(t: number): void {
  const k = smooth((t - (END - OUTRO + 2.2)) / 1.6);
  centered(40, "C R Y S I R R O R", 880, 150, [240, 252, 255, 255], k * 255);
  centered(41, "One engine.  Every story.", 1110, 64, [140, 225, 240, 255], k * 230);
  layers.setOpacity(2, Math.round(255 * smooth((t - (END - 1.6)) / 1.5)));
}

function onUpdate(dtMs: number): void {
  const dt = dtMs / 1000;
  t += dt;
  if (t < INTRO) {
    intro(t);
    return;
  }
  if (logo.visible) {
    logo.visible = false;
    sweep.visible = false;
    if (sparks) sparks.visible = false;
    layers.setOpacity(10, 0);
    layers.setOpacity(11, 0);
  }
  layers.setOpacity(1, Math.round(255 * 0.75 * (1 - smooth((t - INTRO) / 0.45))));
  const { eye, speed } = flyCamera(t, dt);
  hud(t, eye, speed);
  outro(t);
}
