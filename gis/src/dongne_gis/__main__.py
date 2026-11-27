"""명령줄: `uv run python -m dongne_gis build ...` / `... check ...`."""

from __future__ import annotations

import argparse
import shlex
import sys
from pathlib import Path

from .fetch import landmark_query, overpass
from .landmarks import parse_landmarks
from .package import build_package
from .project import LocalProjection


def _area_args(p: argparse.ArgumentParser) -> None:
    p.add_argument("--lat", type=float, required=True, help="중심 위도")
    p.add_argument("--lon", type=float, required=True, help="중심 경도")
    p.add_argument("--size", type=float, default=1500, help="한 변 길이(m)")
    p.add_argument("--cache-dir", type=Path, default=None, help="Overpass 응답 캐시 (기본: 저장소의 data-raw/)")
    p.add_argument("--refresh", action="store_true", help="캐시를 무시하고 다시 받기")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(prog="dongne_gis")
    sub = parser.add_subparsers(dest="cmd", required=True)

    b = sub.add_parser("build", help="OSM에서 맵 패키지 만들기")
    b.add_argument("--name", required=True)
    b.add_argument("--out", type=Path, required=True)
    _area_args(b)

    c = sub.add_parser("check", help="평택역·통복천이 범위 안에 있는지 확인")
    _area_args(c)

    args = parser.parse_args(argv)
    if args.cmd == "build":
        shown = ["uv", "run", "python", "-m", "dongne_gis", "build", "--name", args.name,
                 "--lat", str(args.lat), "--lon", str(args.lon), "--size", f"{args.size:g}",
                 "--out", args.out.as_posix()]
        s = build_package(args.name, args.lat, args.lon, args.size, args.out,
                          shlex.join(shown), args.cache_dir, args.refresh)
        print(f"도로 {s['road_features']}개 (way {s['road_ways']}개), 건물 {s['buildings']}개, "
              f"building:levels {s['buildings_with_levels']}개 ({s['levels_ratio']:.1%}) → {args.out}")
        return 0

    proj = LocalProjection(args.lat, args.lon)
    resp = overpass(landmark_query(proj.bbox(args.size, args.size)), args.cache_dir, args.refresh)
    ok = {"station": False, "stream": False}
    for lm in parse_landmarks(resp, proj):
        xs = [p[0] for p in lm.points]
        ys = [p[1] for p in lm.points]
        inside = lm.inside(args.size, args.size)
        ok[lm.kind] |= inside
        print(f"{lm.kind:7} {lm.osm_type}/{lm.osm_id} {lm.name}: "
              f"x {min(xs):.0f}..{max(xs):.0f}, y {min(ys):.0f}..{max(ys):.0f} m, 범위 안={inside}")
    print(f"평택역 포함={ok['station']}, 통복천 포함={ok['stream']}")
    return 0 if all(ok.values()) else 1


if __name__ == "__main__":
    sys.exit(main())
