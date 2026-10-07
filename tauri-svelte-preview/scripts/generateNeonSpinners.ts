// Generates the owner's 16 neon 3D spinner SVGs (TSK-1391) into
// src/lib/shell/components/conversation/spinners/<id>.svg.
// Every frame is real 3D math (rotate + perspective project), baked into SVG
// <animate> keyframes, so the files need no CSS animation and no JavaScript.
// Run: node --experimental-strip-types scripts/generateNeonSpinners.ts
import { mkdirSync, writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

type Vec3 = [number, number, number];
type Xf = (p: Vec3, t: number) => Vec3;
type Edge = [number, number];
interface Layer { defs: string; glow: string; top: string }

const OUT = fileURLToPath(new URL('../src/lib/shell/components/conversation/spinners/', import.meta.url));
const TAU = Math.PI * 2;
const C = 12, S = 8.2, CAM = 5; // 24x24 viewBox, object radius 1 maps to ~8-10px

// ── 3D helpers ─────────────────────────────────────────────────────────────
const rx = ([x, y, z]: Vec3, a: number): Vec3 => [x, y * Math.cos(a) - z * Math.sin(a), y * Math.sin(a) + z * Math.cos(a)];
const ry = ([x, y, z]: Vec3, a: number): Vec3 => [x * Math.cos(a) + z * Math.sin(a), y, -x * Math.sin(a) + z * Math.cos(a)];
const rz = ([x, y, z]: Vec3, a: number): Vec3 => [x * Math.cos(a) - y * Math.sin(a), x * Math.sin(a) + y * Math.cos(a), z];
const scale = (p: Vec3, f: number): Vec3 => [p[0] * f, p[1] * f, p[2] * f];
const norm = (v: Vec3): Vec3 => scale(v, 1 / Math.hypot(...v));
// Rotate so that unit vector `from` points straight up (+Y).
function alignUp(from: Vec3): (p: Vec3) => Vec3 {
  const [x, y, z] = norm(from);
  const yaw = Math.atan2(x, z); // swing into the YZ plane
  const pitch = Math.atan2(Math.hypot(x, z), y); // then tip up to +Y
  return (p) => rx(ry(p, -yaw), -pitch);
}
const project = ([x, y, z]: Vec3): Vec3 => { const f = CAM / (CAM - z); return [C + S * x * f, C - S * y * f, f]; };
const n = (v: number): string => (+v.toFixed(1)).toString(); // coordinates: 0.1 of a 24px box
const n2 = (v: number): string => (+v.toFixed(2)).toString(); // opacity

// ── SVG builders ───────────────────────────────────────────────────────────
const frames = <T>(count: number, fn: (t: number) => T): T[] => Array.from({ length: count + 1 }, (_, k) => fn(k / count));
const segPath = (pts: Vec3[], edges: Edge[]): string => edges.map(([a, b]) => `M${n(pts[a][0])} ${n(pts[a][1])}L${n(pts[b][0])} ${n(pts[b][1])}`).join('');
const loopPath = (pts: Vec3[]): string => 'M' + pts.map((p) => `${n(p[0])} ${n(p[1])}`).join('L') + 'Z';
const linePath = (pts: Vec3[]): string => 'M' + pts.map((p) => `${n(p[0])} ${n(p[1])}`).join('L');
const anim = (attr: string, values: string[], dur: number): string => `<animate attributeName="${attr}" dur="${dur}s" repeatCount="indefinite" values="${values.join(';')}"/>`;
const core = (hex: string, t = 0.72): string => '#' + [1, 3, 5].map((i) => Math.round(parseInt(hex.slice(i, i + 2), 16) * (1 - t) + 255 * t).toString(16).padStart(2, '0')).join('');

interface StrokeOpts { width?: number; coreWidth?: number; opacity?: number }
// A neon stroke: blurred colored layer underneath, thin bright core on top.
function neonPath(id: string, ds: string[], dur: number, color: string, { width = 1.3, coreWidth = 0.55, opacity = 1 }: StrokeOpts = {}): Layer {
  return {
    defs: `<path id="${id}" d="${ds[0]}">${anim('d', ds, dur)}</path>`,
    glow: `<use href="#${id}" stroke="${color}" stroke-width="${width}" opacity="${opacity}"/>`,
    top: `<use href="#${id}" stroke="${core(color)}" stroke-width="${coreWidth}" opacity="${opacity}"/>`,
  };
}
// A glowing dot that shrinks and dims as it moves away from the viewer.
function neonDot(fr: Vec3[], dur: number, color: string, rBase: number): Layer {
  const cx = fr.map((p) => n(p[0])), cy = fr.map((p) => n(p[1]));
  const op = fr.map((p) => n2(Math.min(1, Math.max(0.3, (p[2] - 0.82) * 3))));
  const dot = (fill: string, size: number): string => {
    const r = fr.map((p) => n(rBase * size * p[2] ** 2));
    return `<circle cx="${cx[0]}" cy="${cy[0]}" r="${r[0]}" fill="${fill}" stroke="none">`
      + anim('cx', cx, dur) + anim('cy', cy, dur) + anim('r', r, dur) + anim('opacity', op, dur) + `</circle>`;
  };
  return { defs: '', glow: dot(color, 1), top: dot(core(color, 0.8), 0.5) };
}
function svg(layers: Layer[], blur = 0.75): string {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke-linecap="round" stroke-linejoin="round">`
    + `<defs><filter id="glow" x="-40%" y="-40%" width="180%" height="180%"><feGaussianBlur stdDeviation="${blur}" result="b"/><feMerge><feMergeNode in="b"/><feMergeNode in="b"/><feMergeNode in="SourceGraphic"/></feMerge></filter>`
    + layers.map((l) => l.defs).join('') + `</defs>`
    + `<g filter="url(#glow)">${layers.map((l) => l.glow).join('')}</g>`
    + layers.map((l) => l.top).join('') + `</svg>`;
}

// Spins `verts` about their own Y axis through `period` radians, then tilts toward the viewer.
function solid(verts: Vec3[], edges: Edge[], { period, dur, color, tilt = 0.42, count = 24 }: { period: number; dur: number; color: string; tilt?: number; count?: number }): string {
  const ds = frames(count, (t) => segPath(verts.map((v) => project(rx(ry(v, period * t), tilt))), edges));
  return svg([neonPath('s', ds, dur, color)]);
}
const edgesAt = (verts: Vec3[], len: number, eps = 1e-6): Edge[] => {
  const out: Edge[] = [];
  for (let i = 0; i < verts.length; i++) for (let j = i + 1; j < verts.length; j++)
    if (Math.abs(Math.hypot(...verts[i].map((c, k) => c - verts[j][k])) - len) < eps) out.push([i, j]);
  return out;
};
const ring = (count: number, map: (a: number) => Vec3): Vec3[] => Array.from({ length: count }, (_, i) => map((i / count) * TAU));

const spinners: { id: string; body: string }[] = [];
const add = (id: string, body: string): void => { spinners.push({ id, body }); };

// ── Set 1: solids ──────────────────────────────────────────────────────────
{ // Cube balanced on a corner
  const raw: Vec3[] = [];
  for (const x of [-1, 1]) for (const y of [-1, 1]) for (const z of [-1, 1]) raw.push([x, y, z]);
  const up = alignUp([1, 1, 1]);
  add('cube', solid(raw.map((p) => up(scale(p, 1 / Math.sqrt(3)))), edgesAt(raw, 2), { period: TAU / 3, dur: 1.4, color: '#00e5ff', tilt: 0.3 }));
}
{ // Octahedron
  const v: Vec3[] = [[1, 0, 0], [-1, 0, 0], [0, 1, 0], [0, -1, 0], [0, 0, 1], [0, 0, -1]];
  add('octahedron', solid(v, edgesAt(v, Math.SQRT2), { period: TAU / 4, dur: 1.0, color: '#ff2bd6' }));
}
{ // Tetrahedron
  const r = Math.sqrt(8 / 9);
  const v: Vec3[] = [[0, 1, 0], ...[0, 1, 2].map((i): Vec3 => [r * Math.cos((i * TAU) / 3), -1 / 3, r * Math.sin((i * TAU) / 3)])];
  add('tetrahedron', solid(v.map((p): Vec3 => [p[0], p[1] * 1.05 + 0.08, p[2]]), [[0, 1], [0, 2], [0, 3], [1, 2], [2, 3], [3, 1]],
    { period: TAU / 3, dur: 1.2, color: '#b4ff00', tilt: 0.38 }));
}
{ // Icosahedron
  const g = (1 + Math.sqrt(5)) / 2;
  const raw: Vec3[] = [];
  for (const a of [-1, 1]) for (const b of [-g, g]) raw.push([0, a, b], [a, b, 0], [b, 0, a]);
  const up = alignUp([0, 1, g]);
  const l = Math.hypot(1, g);
  add('icosahedron', solid(raw.map((p) => up(scale(p, 1 / l))), edgesAt(raw, 2), { period: TAU / 5, dur: 1.1, color: '#a259ff', tilt: 0.25 }));
}
{ // Star tetrahedron (two interlocking tetrahedra, two colors)
  const up = alignUp([1, 1, 1]);
  const a = ([[1, 1, 1], [1, -1, -1], [-1, 1, -1], [-1, -1, 1]] as Vec3[]).map((p) => up(scale(p, 1 / Math.sqrt(3))));
  const b = a.map((p) => scale(p, -1));
  const e: Edge[] = [[0, 1], [0, 2], [0, 3], [1, 2], [2, 3], [3, 1]];
  const mk = (v: Vec3[], t: number): string => segPath(v.map((p) => project(rx(ry(p, (TAU / 3) * t), 0.32))), e);
  add('star', svg([neonPath('a', frames(24, (t) => mk(a, t)), 1.5, '#00e5ff'), neonPath('b', frames(24, (t) => mk(b, t)), 1.5, '#ff2bd6')]));
}
{ // Gyroscope (three nested gimbal rings, each on its own axis)
  const gimbals: { r: number; color: string; spin: Xf }[] = [
    { r: 1, color: '#00e5ff', spin: (p, t) => ry(p, TAU * t) },
    { r: 0.74, color: '#ff2bd6', spin: (p, t) => ry(rx(p, TAU * 2 * t), TAU * t) },
    { r: 0.48, color: '#ffe600', spin: (p, t) => ry(rx(rz(p, TAU * 3 * t), TAU * 2 * t), TAU * t) },
  ];
  const base = (r: number): Vec3[] => ring(20, (a) => [r * Math.cos(a), r * Math.sin(a), 0]);
  const layers = gimbals.map((g, i) => neonPath(`r${i}`, frames(60, (t) => loopPath(base(g.r).map((p) => project(rx(g.spin(p, t), 0.35))))), 3, g.color, { width: 1.15, coreWidth: 0.45 }));
  layers.push({ defs: '', glow: `<circle cx="12" cy="12" r="1.3" fill="#ffffff" stroke="none"/>`, top: '' });
  add('gyroscope', svg(layers));
}
{ // DNA helix
  const R = 0.5, rungs = 7, twist = TAU * 0.9;
  const xf: Xf = (p, t) => rz(rx(ry(p, TAU * t), 0.22), -0.55);
  const strand = (sign: number, t: number): Vec3[] => Array.from({ length: 29 }, (_, i) => {
    const y = -1.05 + (2.1 * i) / 28, a = (twist * i) / 28 + (sign < 0 ? Math.PI : 0);
    return project(xf([R * Math.cos(a), y, R * Math.sin(a)], t));
  });
  const rungAt = (i: number, sign: number): Vec3 => { const y = -0.95 + (1.9 * i) / (rungs - 1), a = (twist * (y + 1.05)) / 2.1 + (sign < 0 ? Math.PI : 0); return [R * Math.cos(a), y, R * Math.sin(a)]; };
  const F = 48, dur = 2.4;
  add('dna', svg([
    neonPath('ra', frames(F, (t) => segPath(Array.from({ length: rungs * 2 }, (_, j) => project(xf(rungAt(j >> 1, j & 1 ? -1 : 1), t))), Array.from({ length: rungs }, (_, i): Edge => [2 * i, 2 * i + 1]))), dur, '#ffffff', { width: 0.5, coreWidth: 0.25, opacity: 0.45 }),
    neonPath('sa', frames(F, (t) => linePath(strand(1, t))), dur, '#2d8bff', { width: 1.1, coreWidth: 0.45 }),
    neonPath('sb', frames(F, (t) => linePath(strand(-1, t))), dur, '#ff3d8b', { width: 1.1, coreWidth: 0.45 }),
  ]));
}

// ── Set 2: the gyroscope family (glowing rings tumbling on their own axes) ──
const circle = (r: number, c: Vec3 = [0, 0, 0], count = 22): Vec3[] => ring(count, (a) => [c[0] + r * Math.cos(a), c[1] + r * Math.sin(a), c[2]]);
const flat = (pts: Vec3[]): Vec3[] => pts.map((p) => rx(p, Math.PI / 2)); // XY ring -> horizontal (XZ) ring
const ROT = { x: rx, y: ry, z: rz };
type Axis = keyof typeof ROT;
// Gimbal chain: ring k turns on `axes[k]` inside ring k-1. Innermost turn is applied first.
const gimbal = (axes: Axis[], rates: number[], k: number): Xf => (p, t) => {
  for (let j = k; j >= 0; j--) p = ROT[axes[j]](p, rates[j] * TAU * t);
  return p;
};
// pts is one loop, or several loops drawn in one path; `open` draws those edges instead of a closed loop.
interface RingItem { pts: Vec3[] | Vec3[][]; xf: Xf; color: string; open?: Edge[]; opts?: StrokeOpts }
interface RingOpts { count?: number; dur?: number; tilt?: number; dots?: { color: string; r?: number; pos: (t: number) => Vec3 }[]; center?: { color: string; r?: number } }
function rings(items: RingItem[], { count = 60, dur = 3, tilt = 0.35, dots = [], center }: RingOpts = {}): string {
  const layers = items.map((it, i) => neonPath(`r${i}`, frames(count, (t) => {
    const loops = (Array.isArray(it.pts[0][0]) ? it.pts : [it.pts]) as Vec3[][];
    return loops.map((pts) => pts.map((p) => project(rx(it.xf(p, t), tilt)))).map((pp) => (it.open ? segPath(pp, it.open) : loopPath(pp))).join('');
  }), dur, it.color, { width: 1.15, coreWidth: 0.45, ...it.opts }));
  if (center) layers.push({ defs: '', glow: `<circle cx="12" cy="12" r="${center.r ?? 1.2}" fill="${center.color}" stroke="none"/>`, top: `<circle cx="12" cy="12" r="${(center.r ?? 1.2) / 2}" fill="#ffffff" stroke="none"/>` });
  for (const d of dots) layers.push(neonDot(frames(count, (t) => project(rx(d.pos(t), tilt))), dur, d.color, d.r ?? 1.1));
  return svg(layers);
}
const NEON = ['#00e5ff', '#ff2bd6', '#ffe600', '#b4ff00', '#a259ff', '#ff9100'];

{ // Armillary sphere
  const A: Xf = (p, t) => ry(p, TAU * t);
  add('gyro-armillary', rings([
    { pts: circle(1), xf: A, color: '#ffb300' },
    { pts: flat(circle(1)), xf: A, color: '#ffb300' },
    { pts: flat(circle(0.84)).map((p) => rz(p, 0.42)), xf: (p, t) => A(ry(p, -2 * TAU * t), t), color: NEON[0] },
    { pts: circle(0.45), xf: (p, t) => A(rx(p, 3 * TAU * t), t), color: NEON[1] },
  ], { count: 64, dur: 4, center: { color: '#ffb300', r: 1 } }));
}
{ // Crossed rings
  const xf: Xf = (p, t) => rz(ry(p, TAU * t), TAU * t);
  add('gyro-cross', rings([{ pts: circle(1), xf, color: NEON[0] }, { pts: circle(1).map((p) => ry(p, Math.PI / 2)), xf, color: NEON[1] }],
    { count: 48, dur: 2.6, center: { color: '#ffffff', r: 0.8 } }));
}
{ // Gimbal with riders
  const axes: Axis[] = ['y', 'x', 'y'], rates = [1, -2, 3], radii = [1, 0.72, 0.44], colors = [NEON[0], NEON[1], NEON[2]];
  add('gyro-riders', rings(radii.map((r, k) => ({ pts: circle(r), xf: gimbal(axes, rates, k), color: colors[k], opts: { opacity: 0.85 } })), {
    count: 64, dur: 4,
    dots: radii.map((r, k) => ({ color: colors[k], pos: (t: number) => gimbal(axes, rates, k)([r * Math.cos(TAU * [2, -3, 4][k] * t + k * 2.1), r * Math.sin(TAU * [2, -3, 4][k] * t + k * 2.1), 0], t) })),
  }));
}
{ // Precession
  add('gyro-precess', rings([1, 0.72, 0.44].map((r, k) => ({ pts: flat(circle(r)).map((p) => rz(p, 0.6)), xf: (p: Vec3, t: number) => ry(p, TAU * t + (k * TAU) / 3), color: [NEON[0], NEON[4], NEON[1]][k] })),
    { count: 48, dur: 2.4, tilt: 0.3, center: { color: NEON[4], r: 0.9 } }));
}
{ // Toy gyroscope: frame ring, spinning rotor with spokes, axle, whole thing precessing
  const A: Xf = (p, t) => ry(rz(p, 0.45), TAU * t);
  const rotor: Xf = (p, t) => A(ry(p, 4 * TAU * t), t);
  const spokes: Vec3[] = [[0, 0, 0], ...[0, 1, 2, 3].map((i): Vec3 => [0.68 * Math.cos((i * TAU) / 4), 0, 0.68 * Math.sin((i * TAU) / 4)])];
  add('gyro-toy', rings([
    { pts: circle(1), xf: A, color: NEON[0] },
    { pts: [[0, -1.18, 0], [0, 1.18, 0]], xf: A, color: '#ffffff', open: [[0, 1]], opts: { width: 0.7, coreWidth: 0.3 } },
    { pts: flat(circle(0.68)), xf: rotor, color: NEON[1] },
    { pts: spokes, xf: rotor, color: NEON[1], open: [[0, 1], [0, 2], [0, 3], [0, 4]], opts: { width: 0.7, coreWidth: 0.3 } },
  ], { count: 60, dur: 3, tilt: 0.3 }));
}
{ // Linked rings
  const xf: Xf = (p, t) => rx(ry(p, TAU * t), TAU * t);
  const r = 0.62;
  add('gyro-linked', rings([
    { pts: circle(r, [-0.3, 0, 0]), xf, color: NEON[0] },
    { pts: ring(22, (a) => [0.3 + r * Math.cos(a), 0, r * Math.sin(a)]), xf, color: NEON[1] },
  ], { count: 48, dur: 3 }));
}
{ // Ripple stack: horizontal rings stacked like a sphere, wobbling in a travelling wave
  const ys = [-0.72, -0.36, 0, 0.36, 0.72];
  const mix = (a: string, b: string, f: number): string => '#' + [1, 3, 5].map((i) => Math.round(parseInt(a.slice(i, i + 2), 16) * (1 - f) + parseInt(b.slice(i, i + 2), 16) * f).toString(16).padStart(2, '0')).join('');
  add('gyro-ripple', rings(ys.map((y, i) => ({
    pts: flat(circle(Math.sqrt(1 - y * y) * 0.95)),
    xf: (p: Vec3, t: number): Vec3 => { const ph = TAU * t - i * 0.9; const q = rx(rz(p, 0.32 * Math.cos(ph)), 0.32 * Math.sin(ph)); return [q[0], q[1] + y, q[2]]; },
    color: mix(NEON[0], NEON[1], i / 4), opts: { width: 1, coreWidth: 0.4 },
  })), { count: 48, dur: 2, tilt: 0.3 }));
}
{ // Hoop torus: eight hoops arranged around a donut, alternating colors
  const hoop = (j: number): Vec3[] => ring(16, (a) => { const b = (j * TAU) / 8, w = 0.6 + 0.36 * Math.cos(a); return [w * Math.cos(b), 0.36 * Math.sin(a), w * Math.sin(b)]; });
  const xf: Xf = (p, t) => ry(p, (TAU / 4) * t);
  add('gyro-torus', rings([0, 1].map((g) => ({ pts: [0, 2, 4, 6].map((j) => hoop(j + g)), xf, color: NEON[g], opts: { width: 1, coreWidth: 0.4 } })),
    { count: 24, dur: 1.2, tilt: 0.75 }));
}
{ // Hoop cascade: five nested hoops on one axis, each at its own speed.
  // A hoop looks the same after half a turn, so rates are in half turns per loop.
  const rates = [1, -1, 2, -2, 3];
  add('gyro-cascade', rings([1, 0.82, 0.64, 0.46, 0.28].map((r, k) => ({ pts: circle(r, [0, 0, 0], 20), xf: (p: Vec3, t: number) => ry(p, Math.PI * rates[k] * t), color: [NEON[0], NEON[4], NEON[1], NEON[5], NEON[2]][k], opts: { width: 1.05, coreWidth: 0.42 } })),
    { count: 36, dur: 1.5, tilt: 0.35, center: { color: '#ffffff', r: 0.7 } }));
}

mkdirSync(OUT, { recursive: true });
for (const s of spinners) writeFileSync(`${OUT}${s.id}.svg`, s.body);
console.log(spinners.map((s) => `${s.id}.svg ${(Buffer.byteLength(s.body) / 1024).toFixed(1)} KB`).join('\n'));
