"""맵 패키지 폴더(manifest.json, roads.geojson, buildings.geojson, README.md)를 만든다."""

from __future__ import annotations

import json
import math
from datetime import datetime, timezone
from pathlib import Path

from . import attribution as attr
from .buildings import build_buildings
from .fetch import map_query, overpass
from .manifest import build_manifest
from .osm import OsmData
from .project import LocalProjection
from .roads import build_roads
from .landcover import build_landcover, landcover_info
from .terrain import build_terrain


def write_geojson(path: Path, features: list[dict]) -> None:
    """한 줄에 피처 하나씩 쓴다 (diff를 읽기 쉽게)."""
    lines = [json.dumps(f, ensure_ascii=False, separators=(",", ":")) for f in features]
    body = ",\n".join(lines)
    path.write_text(
        '{"type":"FeatureCollection","features":[\n' + body + "\n]}\n", encoding="utf-8"
    )


def stats(roads: list[dict], buildings: list[dict]) -> dict:
    road_ways = {f["properties"]["osm_id"] for f in roads}
    with_levels = sum(1 for f in buildings if f["properties"]["building:levels"] is not None)
    with_height = sum(1 for f in buildings if f["properties"]["height"] is not None)
    n = len(buildings)
    return {
        "road_features": len(roads),
        "road_ways": len(road_ways),
        "buildings": n,
        "buildings_with_levels": with_levels,
        "levels_ratio": with_levels / n if n else 0.0,
        "buildings_with_height": with_height,
    }


def build_package(
    name: str,
    lat: float,
    lon: float,
    size_m: float,
    out: Path,
    command: str,
    cache_dir: Path | None = None,
    refresh: bool = False,
    terrain: bool = True,
) -> dict:
    proj = LocalProjection(lat, lon)
    bbox = proj.bbox(size_m, size_m)
    osm = OsmData.from_overpass(overpass(map_query(bbox), cache_dir, refresh))

    roads = build_roads(osm, proj, size_m, size_m)
    buildings = build_buildings(osm, proj, size_m, size_m)

    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    size_out = int(size_m) if float(size_m).is_integer() else size_m
    lo = hi = 0.0
    if terrain:
        _, lo, hi = build_terrain(out, lat, lon, size_m, size_m, cache_dir=cache_dir)
    lc = build_landcover(out, osm, proj, size_m, size_m)
    manifest = build_manifest(name, lat, lon, size_out, size_out, lo, hi, landcover=landcover_info())
    (out / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    write_geojson(out / "roads.geojson", roads)
    write_geojson(out / "buildings.geojson", buildings)

    s = stats(roads, buildings)
    s["osm_timestamp"] = osm.timestamp
    s["bbox"] = bbox
    s["terrain"] = terrain
    s["min_elevation_m"], s["max_elevation_m"] = lo, hi
    s["landcover_mean"] = {k: float(v.mean()) for k, v in lc.items()}
    s["landcover"] = lc
    (out / "README.md").write_text(_readme(name, lat, lon, size_m, command, s), encoding="utf-8")
    return s


def _readme(name: str, lat: float, lon: float, size_m: float, command: str, s: dict) -> str:
    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    south, west, north, east = s["bbox"]
    lcm = s["landcover_mean"]
    if s["terrain"]:
        terrain_line = (
            f"16비트 그레이스케일 높이맵 {math.ceil(size_m / 30)}×{math.ceil(size_m / 30)} (30m 셀, 행 0 = 북쪽). "
            f"픽셀 0~65535 = 고도 {s['min_elevation_m']:g}~{s['max_elevation_m']:g} m. "
            "Copernicus DEM은 지표면 모델(DSM)이라 건물·나무 높이가 섞여 있음"
        )
    else:
        terrain_line = "만들지 않음 (`--no-terrain`)"
    return f"""# {name}

샘플 맵 패키지입니다. 근거: [docs/design/sample-area.md](../../docs/design/sample-area.md)

## 출처와 라이선스

도로·건물·토지 피복(`roads.geojson`, `buildings.geojson`, `landcover.png`)은 OpenStreetMap에서 만든 파생 데이터베이스이며
[Open Database License (ODbL) 1.0](https://opendatacommons.org/licenses/odbl/1-0/)로 배포합니다.

© OpenStreetMap contributors — https://www.openstreetmap.org/copyright

지형(`terrain.png`)은 Copernicus DEM GLO-30에서 만들었습니다.

{attr.COPERNICUS_DEM}

## 범위

| 항목 | 값 |
| --- | --- |
| 원점 (origin) | 북위 {lat}, 동경 {lon} |
| 크기 | {size_m:g} m × {size_m:g} m (origin 중심) |
| 받은 범위 (bbox) | S {south:.6f}, W {west:.6f}, N {north:.6f}, E {east:.6f} |
| 좌표 | origin 중심 횡메르카토르, 미터 (x 동쪽, y 북쪽), 소수 2자리 |

## 내용

| 파일 | 내용 |
| --- | --- |
| `manifest.json` | 메타데이터 (origin, 범위, 지형 고도 범위, 토지 피복 채널, 출처) |
| `terrain.png` | {terrain_line} |
| `landcover.png` | 8비트 RGB 토지 피복 (terrain과 같은 격자). R = 불투수, G = 녹지·나무 수관, B = 수면, 0~255 = 셀 면적 비율 0~1. OSM 도형에서 추정 (구역 평균: 불투수 {lcm["impervious"]:.1%}, 녹지 {lcm["canopy"]:.1%}, 수면 {lcm["water"]:.1%}) |
| `roads.geojson` | 도로 LineString {s["road_features"]}개 (OSM way {s["road_ways"]}개, 범위 경계에서 자름) |
| `buildings.geojson` | 건물 Polygon {s["buildings"]}개 (무게중심이 범위 안인 건물, 자르지 않음) |

건물 층수(`building:levels`)가 있는 건물: {s["buildings_with_levels"]}개 ({s["levels_ratio"]:.1%}), 높이(`height`)가 있는 건물: {s["buildings_with_height"]}개.

## 만든 방법

```sh
cd gis
{command}
```

- 생성 일시: {now}
- OSM 데이터 기준 시각 (Overpass): {s["osm_timestamp"]}
"""
