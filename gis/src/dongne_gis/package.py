"""맵 패키지 폴더(manifest.json, roads.geojson, buildings.geojson, README.md)를 만든다."""

from __future__ import annotations

import json
from datetime import datetime, timezone
from pathlib import Path

from .buildings import build_buildings
from .fetch import map_query, overpass
from .manifest import build_manifest
from .osm import OsmData
from .project import LocalProjection
from .roads import build_roads


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
) -> dict:
    proj = LocalProjection(lat, lon)
    bbox = proj.bbox(size_m, size_m)
    osm = OsmData.from_overpass(overpass(map_query(bbox), cache_dir, refresh))

    roads = build_roads(osm, proj, size_m, size_m)
    buildings = build_buildings(osm, proj, size_m, size_m)

    out = Path(out)
    out.mkdir(parents=True, exist_ok=True)
    # 지형은 이슈 #2에서 채운다. 그때까지 고도 범위는 0으로 둔다.
    size_out = int(size_m) if float(size_m).is_integer() else size_m
    manifest = build_manifest(name, lat, lon, size_out, size_out, 0, 0)
    (out / "manifest.json").write_text(
        json.dumps(manifest, ensure_ascii=False, indent=2) + "\n", encoding="utf-8"
    )
    write_geojson(out / "roads.geojson", roads)
    write_geojson(out / "buildings.geojson", buildings)

    s = stats(roads, buildings)
    s["osm_timestamp"] = osm.timestamp
    s["bbox"] = bbox
    (out / "README.md").write_text(_readme(name, lat, lon, size_m, command, s), encoding="utf-8")
    return s


def _readme(name: str, lat: float, lon: float, size_m: float, command: str, s: dict) -> str:
    now = datetime.now(timezone.utc).strftime("%Y-%m-%d %H:%M UTC")
    south, west, north, east = s["bbox"]
    return f"""# {name}

샘플 맵 패키지입니다. 근거: [docs/design/sample-area.md](../../docs/design/sample-area.md)

## 출처와 라이선스

이 폴더의 데이터는 OpenStreetMap에서 만든 파생 데이터베이스이며
[Open Database License (ODbL) 1.0](https://opendatacommons.org/licenses/odbl/1-0/)로 배포합니다.

© OpenStreetMap contributors — https://www.openstreetmap.org/copyright

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
| `manifest.json` | 메타데이터. `terrain` 블록은 자리만 잡아 둔 값(고도 0~0)이고, `terrain.png`와 실제 고도 범위는 이슈 #2에서 채움 |
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
