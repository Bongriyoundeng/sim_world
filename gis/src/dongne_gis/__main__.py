"""명령줄: `uv run python -m dongne_gis build ...` / `... check ...`."""

from __future__ import annotations

import argparse
import shlex
import sys
from pathlib import Path

import numpy as np

from .fetch import landmark_query, map_query, overpass
from .landcover import collect, coverage
from .landmarks import parse_landmarks
from .osm import OsmData
from .package import build_package
from .project import LocalProjection
from .terrain import cells_on_lines, fetch_tile, resample, tiles_for_bbox


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
    b.add_argument("--no-terrain", action="store_true", help="지형(terrain.png) 단계 건너뛰기")
    _area_args(b)

    c = sub.add_parser("check", help="평택역·통복천이 범위 안에 있는지 확인")
    _area_args(c)

    r = sub.add_parser("register", help="건축물대장 표제부 → 법정동 건물 통계 (키: DATA_GO_KR_KEY)")
    r.add_argument("--sigungu", required=True, help="시군구 코드 (예: 41220 평택시)")
    r.add_argument("--bjdong", nargs="+", required=True, help="법정동 코드 (예: 11300 평택동)")
    r.add_argument("--out", type=Path, help="통계 JSON을 저장할 파일")
    r.add_argument("--cache-dir", type=Path, default=None)
    r.add_argument("--refresh", action="store_true")

    args = parser.parse_args(argv)
    if args.cmd == "register":
        import json

        from .building_register import RegisterError, fetch_dong, summarize

        items = []
        try:
            for d in args.bjdong:
                got = fetch_dong(args.sigungu, d, args.cache_dir, args.refresh)
                print(f"{args.sigungu}-{d}: 표제부 {len(got)}건")
                items += got
        except RegisterError as e:
            print(f"건축물대장 오류: {e}", file=sys.stderr)
            return 2
        stats = summarize(items)
        stats["sigungu"], stats["bjdong"] = args.sigungu, args.bjdong
        text = json.dumps(stats, ensure_ascii=False, indent=2)
        print(text)
        if args.out:
            args.out.parent.mkdir(parents=True, exist_ok=True)
            args.out.write_text(text + "\n", encoding="utf-8")
        return 0
    if args.cmd == "build":
        shown = ["uv", "run", "python", "-m", "dongne_gis", "build", "--name", args.name,
                 "--lat", str(args.lat), "--lon", str(args.lon), "--size", f"{args.size:g}",
                 "--out", args.out.as_posix()]
        s = build_package(args.name, args.lat, args.lon, args.size, args.out,
                          shlex.join(shown), args.cache_dir, args.refresh,
                          terrain=not args.no_terrain)
        print(f"도로 {s['road_features']}개 (way {s['road_ways']}개), 건물 {s['buildings']}개, "
              f"building:levels {s['buildings_with_levels']}개 ({s['levels_ratio']:.1%}), "
              f"고도 {s['min_elevation_m']}~{s['max_elevation_m']} m → {args.out}")
        lcm = s["landcover_mean"]
        print(f"토지 피복 평균: 불투수 {lcm['impervious']:.3f}, 녹지 {lcm['canopy']:.3f}, 수면 {lcm['water']:.3f}")
        return 0

    proj = LocalProjection(args.lat, args.lon)
    resp = overpass(landmark_query(proj.bbox(args.size, args.size)), args.cache_dir, args.refresh)
    ok = {"station": False, "stream": False}
    landmarks = parse_landmarks(resp, proj)
    for lm in landmarks:
        xs = [p[0] for p in lm.points]
        ys = [p[1] for p in lm.points]
        inside = lm.inside(args.size, args.size)
        ok[lm.kind] |= inside
        print(f"{lm.kind:7} {lm.osm_type}/{lm.osm_id} {lm.name}: "
              f"x {min(xs):.0f}..{max(xs):.0f}, y {min(ys):.0f}..{max(ys):.0f} m, 범위 안={inside}")
    print(f"평택역 포함={ok['station']}, 통복천 포함={ok['stream']}")

    # 통복천이 지나는 셀의 평균 고도와 전체 평균 고도 비교 (저지대 확인)
    tiles = [fetch_tile(t, args.cache_dir) for t in tiles_for_bbox(proj.bbox(args.size, args.size))]
    heights = resample(tiles, args.lat, args.lon, args.size, args.size)
    stream = cells_on_lines([lm.points for lm in landmarks if lm.kind == "stream"], args.size, args.size, 30)
    if stream:
        on = float(np.mean([heights[r, c] for r, c in stream]))
        print(f"고도: 전체 평균 {heights.mean():.1f} m (최저 {heights.min():.1f}, 최고 {heights.max():.1f}), "
              f"통복천 셀 {len(stream)}개 평균 {on:.1f} m, 차이 {on - heights.mean():+.1f} m")
        # 통복천 셀의 수면 비율 (토지 피복)
        osm = OsmData.from_overpass(overpass(map_query(proj.bbox(args.size, args.size)), args.cache_dir))
        water = coverage(collect(osm, proj).water, args.size, args.size, 30)
        ws = [water[r, c] for r, c in stream]
        print(f"수면 비율: 통복천 셀 평균 {np.mean(ws):.2f} (0.5 이상 셀 {sum(w >= 0.5 for w in ws)}/{len(ws)}), "
              f"구역 평균 {water.mean():.3f}")
    return 0 if all(ok.values()) else 1


if __name__ == "__main__":
    sys.exit(main())
