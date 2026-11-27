"""맵 패키지 manifest.json (v0) 생성. 형식은 schema/map-package/README.md 참고."""

from __future__ import annotations

from . import attribution as attr

FORMAT_VERSION = 0


def build_manifest(
    name: str,
    lat: float,
    lon: float,
    width_m: float,
    height_m: float,
    min_elevation_m: float,
    max_elevation_m: float,
    resolution_m: float = 30,
    attribution: list[str] | None = None,
    landcover: dict | None = None,
) -> dict:
    sources = attribution if attribution is not None else list(attr.DEFAULT)
    if not any(s.strip() for s in sources):
        raise ValueError("attribution(출처)이 비어 있으면 맵 패키지를 만들 수 없습니다")
    m = {
        "format_version": FORMAT_VERSION,
        "name": name,
        "origin": {"lat": lat, "lon": lon},
        "crs": "local-meters",
        "extent_m": {"width": width_m, "height": height_m},
        "terrain": {
            "file": "terrain.png",
            "resolution_m": resolution_m,
            "min_elevation_m": min_elevation_m,
            "max_elevation_m": max_elevation_m,
        },
        "attribution": sources,
    }
    if landcover is not None:
        m["landcover"] = landcover  # 선택 항목 (schema/map-package/README.md)
    return m
