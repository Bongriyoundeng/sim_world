"""클라이언트 데모 프레임(PNG)을 README용 GIF로 묶는다.

    # 1) 프레임 녹화 (창이 잠깐 열린다)
    godot --path client --resolution 1280x720 -- --demo demo_frames --demo-frames 240
    # 2) GIF 만들기 (Pillow 필요: uv pip install pillow)
    python tools/make_demo_gif.py demo_frames docs/images/demo.gif --width 640 --every 2 --fps 15

데모 장면: tests/scenarios/scenario.py와 같은 구역, 출퇴근 1,690명, 07:36부터 60배속.
GIF에도 지도 데이터 출처(OSM·Copernicus) 표기가 필요하다 — README의 GIF 아래에 적는다.
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from PIL import Image


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("frames", type=Path, help="frame_0000.png … 가 있는 폴더")
    p.add_argument("out", type=Path, help="만들 GIF 경로")
    p.add_argument("--width", type=int, default=640)
    p.add_argument("--every", type=int, default=2, help="n장마다 한 장 사용")
    p.add_argument("--fps", type=int, default=15)
    p.add_argument("--colors", type=int, default=128)
    args = p.parse_args()

    files = sorted(args.frames.glob("frame_*.png"))[:: args.every]
    if not files:
        print(f"{args.frames}에 frame_*.png가 없음", file=sys.stderr)
        return 1
    frames = []
    for f in files:
        img = Image.open(f).convert("RGB")
        h = round(img.height * args.width / img.width)
        frames.append(img.resize((args.width, h), Image.LANCZOS))
    # 첫 프레임으로 만든 팔레트를 모든 프레임에 써서 깜빡임을 줄인다
    palette = frames[0].quantize(colors=args.colors, method=Image.MEDIANCUT)
    quantized = [fr.quantize(palette=palette, dither=Image.Dither.NONE) for fr in frames]
    args.out.parent.mkdir(parents=True, exist_ok=True)
    quantized[0].save(
        args.out,
        save_all=True,
        append_images=quantized[1:],
        duration=round(1000 / args.fps),
        loop=0,
        optimize=True,
    )
    size = args.out.stat().st_size
    print(f"{len(quantized)}프레임, {quantized[0].size[0]}×{quantized[0].size[1]}, {size / 1e6:.1f} MB → {args.out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
