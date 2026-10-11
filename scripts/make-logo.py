#!/usr/bin/env python3
"""Renders the animated Tandem logo: six collaborators' cursors write T-A-N-D-E-M
together, blink, erase the word and start again.

Writes a light and a dark GIF (GIF transparency is 1-bit, so the anti-aliased
edges are blended against each theme's background instead).

    python3 scripts/make-logo.py [--font path/to/NotoSans[wght].ttf]

Needs Pillow and the Noto Sans variable font (the app's font).
"""
import argparse
import math
import shutil
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

ROOT = Path(__file__).resolve().parent.parent
OUT_DIRS = [ROOT / "srcs/requirements/frontend/public", ROOT / "DOCS/img"]

WORD = "TANDEM"
# one colour per collaborator, starting from the app's primary green
COLORS = ["#547a5c", "#d97706", "#2563eb", "#db2777", "#7c3aed", "#0d9488"]
THEMES = {
	"light": {"bg": "#ffffff", "ink": "#292524"},
	"dark": {"bg": "#0c0a09", "ink": "#f5f5f4"},
}

SCALE = 2  # rendered at 2x so it stays sharp on high-density screens
FONT_SIZE = 64 * SCALE
GAP = 14 * SCALE  # space between letters, room for a cursor
PAD_X, PAD_TOP, PAD_BOTTOM = 18 * SCALE, 22 * SCALE, 14 * SCALE
CARET_W = 3 * SCALE
FLAG_W, FLAG_H = 10 * SCALE, 6 * SCALE

FPS = 25
FRAME_MS = 1000 // FPS
# timeline (seconds)
APPEAR = 0.6  # cursors blink at their starting positions
STAGGER = 0.16  # each writer starts a little after the previous one
WRITE = 0.45  # time to write one letter
HOLD = 3.2  # word stays, cursors blink
ERASE_STAGGER = 0.07
ERASE = 0.18
REST = 0.4  # empty pause before looping
BLINK = 0.53  # half period, like an editor caret


def ease(t: float) -> float:
	return 0.5 - math.cos(math.pi * max(0.0, min(1.0, t))) / 2


def build(font_path: str):
	font = ImageFont.truetype(font_path, FONT_SIZE)
	try:
		font.set_variation_by_axes([700])
	except OSError:
		pass  # static bold font

	# letter boxes from the font's real glyph metrics
	ascent, descent = font.getmetrics()
	widths = [font.getbbox(ch)[2] - font.getbbox(ch)[0] for ch in WORD]
	offsets = [-font.getbbox(ch)[0] for ch in WORD]
	x, lefts = PAD_X, []
	for w in widths:
		lefts.append(x)
		x += w + GAP
	width = x - GAP + PAD_X
	cap_top = font.getbbox("T")[1]
	cap_bottom = font.getbbox("T")[3]
	height = PAD_TOP + (cap_bottom - cap_top) + PAD_BOTTOM
	text_y = PAD_TOP - cap_top
	caret_top, caret_bottom = PAD_TOP - 4 * SCALE, PAD_TOP + (cap_bottom - cap_top) + 4 * SCALE

	write_end = APPEAR + STAGGER * (len(WORD) - 1) + WRITE
	erase_start = write_end + HOLD
	erase_end = erase_start + ERASE_STAGGER * (len(WORD) - 1) + ERASE
	total = erase_end + REST

	# blink phases spread evenly over the on/off cycle (and shuffled across the word),
	# so the cursors look independent and about half are always visible
	phases = [0, 4, 2, 5, 1, 3]

	def blinking(i: int, t: float) -> bool:
		return int(t / BLINK + phases[i] * 2 / len(WORD)) % 2 == 0

	def state(i: int, t: float):
		"""(revealed fraction of letter i, caret visible?)"""
		start = APPEAR + STAGGER * i
		if t < start:  # waiting at the start of its slot
			return 0.0, blinking(i, t)
		if t < start + WRITE:  # writing: caret solid
			return ease((t - start) / WRITE), True
		e_start = erase_start + ERASE_STAGGER * (len(WORD) - 1 - i)  # erased right to left
		if t < e_start:
			return 1.0, blinking(i, t)
		if t < e_start + ERASE:
			return 1.0 - (t - e_start) / ERASE, True
		return 0.0, t < erase_end

	def frame(theme: dict, t: float) -> Image.Image:
		img = Image.new("RGB", (width, height), theme["bg"])
		draw = ImageDraw.Draw(img)
		for i, ch in enumerate(WORD):
			revealed, caret = state(i, t)
			left, w = lefts[i], widths[i]
			if revealed > 0:
				# draw the glyph on its own layer and keep only the written part
				layer = Image.new("L", (width, height), 0)
				ImageDraw.Draw(layer).text((left + offsets[i], text_y), ch, font=font, fill=255)
				mask = Image.new("L", (width, height), 0)
				ImageDraw.Draw(mask).rectangle([0, 0, left + w * revealed, height], fill=255)
				img.paste(theme["ink"], (0, 0), Image.composite(layer, Image.new("L", (width, height), 0), mask))
			if caret:
				cx = left + w * revealed + (3 * SCALE if revealed > 0 else -5 * SCALE)
				draw.rectangle([cx, caret_top, cx + CARET_W - 1, caret_bottom], fill=COLORS[i])
				draw.rounded_rectangle(
					[cx, caret_top - FLAG_H, cx + FLAG_W, caret_top + 1 * SCALE], radius=2 * SCALE, fill=COLORS[i]
				)
		return img

	n = int(total * FPS)
	return {name: [frame(theme, k / FPS) for k in range(n)] for name, theme in THEMES.items()}


def save(frames, path: Path):
	# merge identical consecutive frames into one longer frame
	merged, durations = [], []
	for f in frames:
		if merged and f.tobytes() == merged[-1].tobytes():
			durations[-1] += FRAME_MS
		else:
			merged.append(f)
			durations.append(FRAME_MS)
	palette = [f.quantize(colors=64, method=Image.Quantize.MEDIANCUT, dither=Image.Dither.NONE) for f in merged]
	palette[0].save(path, save_all=True, append_images=palette[1:], duration=durations, loop=0, optimize=True, disposal=1)
	return len(merged)


if __name__ == "__main__":
	parser = argparse.ArgumentParser()
	parser.add_argument("--font", default="/usr/share/fonts/google-noto-vf/NotoSans[wght].ttf")
	args = parser.parse_args()
	for name, frames in build(args.font).items():
		target = OUT_DIRS[0] / f"logo-{name}.gif"
		count = save(frames, target)
		for extra in OUT_DIRS[1:]:
			shutil.copy(target, extra / target.name)
		print(f"{target.relative_to(ROOT)}: {frames[0].size[0]}x{frames[0].size[1]}, {count} frames, {target.stat().st_size // 1024} KB")
