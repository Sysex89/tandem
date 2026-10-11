<template>
  <canvas ref="canvas" class="absolute inset-0 w-full h-full cursor-pointer bg-white dark:bg-surface-950" />
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';

const canvas = ref<HTMLCanvasElement | null>(null);
let animId: number;
let resizeHandler: (() => void) | null = null;
let clickHandler: ((e: MouseEvent) => void) | null = null;
let mountedEl: HTMLCanvasElement | null = null;

const COLORS_DARK = [
  'rgba(107, 148, 116,',
  'rgba(168, 194, 175,',
  'rgba(212, 184, 150,',
  'rgba(180, 160, 130,',
  'rgba(200, 200, 190,',
];

const COLORS_LIGHT = [
  'rgba(60, 100, 70,',
  'rgba(80, 130, 90,',
  'rgba(150, 110, 60,',
  'rgba(120, 100, 70,',
  'rgba(100, 110, 90,',
];

type Ring = { radius: number; dotCount: number; colorIdx: number; offset: number };

type Cell = {
  x: number;
  y: number;
  rings: Ring[];
  phase: number;
  speed: number;
  scale: number;
  bornAt: number;
};

type Thread = {
  dots: { x: number; y: number }[];
  colorIdx: number;
  interval: number;
  startAt: number;
  bounded: boolean;
};

function makeRing(i: number): Ring {
  return {
    radius: 14 + i * 16 + Math.random() * 6,
    dotCount: 16 + i * 8,
    colorIdx: Math.floor(Math.random() * COLORS_DARK.length),
    offset: Math.random() * Math.PI * 2,
  };
}

function makeCell(w: number, h: number, bornAt: number): Cell {
  const ringCount = 1 + Math.floor(Math.random() * 4);
  return {
    x: Math.random() * w,
    y: Math.random() * h,
    rings: Array.from({ length: ringCount }, (_, i) => makeRing(i)),
    phase: Math.random() * Math.PI * 2,
    speed: 0.0003 + Math.random() * 0.0003,
    scale: 0.5 + Math.random() * 0.8,
    bornAt,
  };
}

function makeWalk(
  sx: number, sy: number,
  w: number, h: number,
  bounded: boolean,
): { x: number; y: number }[] {
  const cx = w / 2, cy = (h * 2) / 3;
  const dots: { x: number; y: number }[] = [];
  let x = sx, y = sy;

  const outwardAngle = Math.atan2(sy - cy, sx - cx);
  let angle = bounded
    ? Math.random() * Math.PI * 2
    : outwardAngle + (Math.random() - 0.5) * 1.0;

  for (let i = 0; i < 700; i++) {
    const toCenterAngle = Math.atan2(cy - y, cx - x);
    let diff = toCenterAngle - angle;
    if (diff > Math.PI) diff -= 2 * Math.PI;
    if (diff < -Math.PI) diff += 2 * Math.PI;

    const pull = bounded ? 0.05 : 0;
    angle += diff * pull + (Math.random() - 0.5) * 0.75;

    const step = 9 + Math.random() * 5;
    x += Math.cos(angle) * step;
    y += Math.sin(angle) * step;

    if (bounded) {
      if (x < 20) { x = 20; angle = Math.PI - angle; }
      if (x > w - 20) { x = w - 20; angle = Math.PI - angle; }
      if (y < 20) { y = 20; angle = -angle; }
      if (y > h - 20) { y = h - 20; angle = -angle; }
    }

    dots.push({ x: x + (Math.random() - 0.5) * 0.8, y: y + (Math.random() - 0.5) * 0.8 });
  }
  return dots;
}

function spawnThreads(cell: Cell, w: number, h: number, cursorStart: number): { threads: Thread[]; cursorEnd: number } {
  const threads: Thread[] = [];
  const perCell = 2 + Math.floor(Math.random() * 2);
  let cursor = cursorStart;
  for (let k = 0; k < perCell; k++) {
    const exitAngle = Math.random() * Math.PI * 2;
    const exitR = 20 + Math.random() * 15;
    const bounded = Math.random() > 0.2;
    threads.push({
      dots: makeWalk(cell.x + Math.cos(exitAngle) * exitR, cell.y + Math.sin(exitAngle) * exitR, w, h, bounded),
      colorIdx: Math.floor(Math.random() * COLORS_DARK.length),
      interval: 0.08 + Math.random() * 0.12,
      startAt: cursor,
      bounded,
    });
    cursor += 2.5 + Math.random() * 2;
  }
  return { threads, cursorEnd: cursor };
}

const FPS = 30;
const INTERVAL = 1000 / FPS;
let lastFrame = 0;

function draw(
  ctx: CanvasRenderingContext2D,
  threadCtx: CanvasRenderingContext2D,
  threadCanvas: HTMLCanvasElement,
  cells: Cell[],
  threads: Thread[],
  threadDrawn: number[],
  ts: number,
  dark: boolean,
) {
  const { width: w, height: h } = ctx.canvas;
  const colors = dark ? COLORS_DARK : COLORS_LIGHT;
  ctx.clearRect(0, 0, w, h);

  // Cells — unchanged
  for (const cell of cells) {
    const age = ts - cell.bornAt;
    const growIn = age < 3 ? age / 3 : 1;
    const pulse = 0.88 + 0.12 * Math.sin(ts * cell.speed * 1000 + cell.phase);
    const s = cell.scale * growIn;

    for (const ring of cell.rings) {
      const r = ring.radius * s * pulse;
      const color = colors[ring.colorIdx];
      for (let i = 0; i < ring.dotCount; i++) {
        const angle = ring.offset + (i / ring.dotCount) * Math.PI * 2 + ts * cell.speed;
        const dx = cell.x + Math.cos(angle) * r;
        const dy = cell.y + Math.sin(angle) * r;
        const dotR = Math.max((2.0 - ring.radius / 70) * s * pulse, 0.5);
        const alphaMod = (Math.sin(angle * 2 + ts * cell.speed * 3) + 1) / 2;
        const alpha = (dark ? 0.14 + 0.20 * alphaMod : 0.58 + 0.22 * alphaMod) * growIn;
        ctx.beginPath();
        ctx.arc(dx, dy, dotR, 0, Math.PI * 2);
        ctx.fillStyle = `${color}${alpha})`;
        ctx.fill();
      }
    }
  }

  // Threads — only paint dots that have never been painted before
  threadCtx.globalCompositeOperation = dark ? 'lighter' : 'multiply';
  for (const [t, thread] of threads.entries()) {
    const drawn = threadDrawn[t] ?? 0;
    const elapsed = ts - thread.startAt;
    if (elapsed < 0) continue;
    const visible = Math.min(Math.floor(elapsed / thread.interval), thread.dots.length);
    if (visible <= drawn) continue; // nothing new to paint

    const color = colors[thread.colorIdx];
    const baseAlpha = dark ? 0.14 : 0.22;

    for (let k = drawn; k < visible; k++) {
      const dot = thread.dots[k];
      if (!dot) break; // k < visible <= dots.length, so this never triggers
      // alphaMod baked in using position so the variation is stable
      const alphaMod = (Math.sin(dot.x * 0.05 + dot.y * 0.03) + 1) / 2;
      const alpha = baseAlpha * (0.7 + 0.3 * alphaMod);
      threadCtx.fillStyle = `${color}${alpha})`;
      threadCtx.beginPath();
      threadCtx.arc(dot.x, dot.y, 1.5, 0, Math.PI * 2);
      threadCtx.fill();
    }
    threadDrawn[t] = visible;
  }

  // Stamp the offscreen thread canvas onto the main canvas in one go
  ctx.globalCompositeOperation = dark ? 'lighter' : 'multiply';
  ctx.drawImage(threadCanvas, 0, 0);
  ctx.globalCompositeOperation = 'source-over';
}

onMounted(() => {
  const el = canvas.value!;
  mountedEl = el;
  const ctx = el.getContext('2d')!;

  // Hidden canvas — thread dots live here permanently
  const threadCanvas = document.createElement('canvas');
  const threadCtx = threadCanvas.getContext('2d')!;
  // Tracks how many dots of each thread have been painted so far
  const threadDrawn: number[] = [];

  let wasDark = document.documentElement.classList.contains('dark');

  const resize = () => {
    el.width = el.offsetWidth;
    el.height = el.offsetHeight;
    threadCanvas.width = el.width;
    threadCanvas.height = el.height;
    // Resizing a canvas wipes it, so reset counters to repaint everything
    threadDrawn.fill(0);
  };
  resizeHandler = resize;
  resize();
  window.addEventListener('resize', resize);

  const cells: Cell[] = Array.from({ length: 6 }, () => makeCell(el.width, el.height, 0));
  const threads: Thread[] = [];

  let cursor = 1;
  for (const cell of cells) {
    const result = spawnThreads(cell, el.width, el.height, cursor);
    threads.push(...result.threads);
    result.threads.forEach(() => threadDrawn.push(0)); // one counter per thread
    cursor = result.cursorEnd;
  }

  let nextSpawnAt = 90 + Math.random() * 90;

  const onClick = (e: MouseEvent) => {
    const rect = el.getBoundingClientRect();
    const mx = (e.clientX - rect.left) * (el.width / rect.width);
    const my = (e.clientY - rect.top) * (el.height / rect.height);
    let closest: Cell | null = null;
    let closestDist = Infinity;
    for (const cell of cells) {
      const d = Math.hypot(mx - cell.x, my - cell.y);
      if (d < closestDist) { closestDist = d; closest = cell; }
    }
    if (!closest || closestDist > 60) return;
    const gain = Math.random() < 0.5;
    if (gain && closest.rings.length < 5) {
      closest.rings.push(makeRing(closest.rings.length));
    } else if (!gain && closest.rings.length > 1) {
      closest.rings.pop();
    } else if (closest.rings.length < 5) {
      closest.rings.push(makeRing(closest.rings.length));
    }
  };
  clickHandler = onClick;
  el.addEventListener('click', onClick);

  const loop = (ts: number) => {
    animId = requestAnimationFrame(loop);
    if (ts - lastFrame < INTERVAL) return;
    lastFrame = ts;
    const tsSeconds = ts / 1000;
    const dark = document.documentElement.classList.contains('dark');

    // Light/dark switched — wipe offscreen canvas and repaint all dots next frame
    if (dark !== wasDark) {
      threadCtx.clearRect(0, 0, threadCanvas.width, threadCanvas.height);
      threadCtx.globalCompositeOperation = dark ? 'lighter' : 'multiply';
      threadDrawn.fill(0);
      wasDark = dark;
    }

    if (tsSeconds > nextSpawnAt) {
      const newCell = makeCell(el.width, el.height, tsSeconds);
      cells.push(newCell);
      const result = spawnThreads(newCell, el.width, el.height, tsSeconds + 1);
      threads.push(...result.threads);
      result.threads.forEach(() => threadDrawn.push(0)); // one counter per new thread
      nextSpawnAt = tsSeconds + 90 + Math.random() * 90;
    }

    draw(ctx, threadCtx, threadCanvas, cells, threads, threadDrawn, tsSeconds, dark);
  };
  animId = requestAnimationFrame(loop);
});

onUnmounted(() => {
  cancelAnimationFrame(animId);
  if (resizeHandler) {
    window.removeEventListener('resize', resizeHandler);
  }
  if (mountedEl && clickHandler) {
    mountedEl.removeEventListener('click', clickHandler);
  }
  resizeHandler = null;
  clickHandler = null;
  mountedEl = null;
});
</script>