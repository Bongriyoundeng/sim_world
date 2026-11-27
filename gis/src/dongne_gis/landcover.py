"""OSM → 토지 피복 레이어 landcover.png (30m 격자, 8비트 RGB).

채널 (0~255 = 셀 면적 대비 비율 0~1):
- R impervious: 건물 지붕 + 도로(선을 유형별 폭으로 버퍼) + 포장 면적(주차장 등)
- G canopy: 나무·숲·잔디·공원 등 녹지와 나무 수관
- B water: 수면(natural=water 등)과 하천 선 버퍼

격자는 terrain.png와 같습니다 (행 0 = 북쪽, 열 0 = 서쪽). 폭·반경 기본값의 근거는
docs/design/research-log.md#road-width 를 보세요. "가정"이라고 적힌 값은 근거 없이 정한 값입니다.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

import numpy as np
from shapely import STRtree
from shapely.geometry import LineString, Point, Polygon, box
from shapely.geometry.base import BaseGeometry
from shapely.ops import unary_union

from .buildings import _relation_polygons
from .osm import OsmData, parse_number
from .project import LocalProjection
from .terrain import grid_shape

# 도로의 구조·시설 기준에 관한 규칙 제10조 ③ 표: 도시지역 차로 최소 폭.
# 설계속도 80 km/h 이상 3.25 m, 80 미만 3.00 m. 원도심 도로는 80 미만으로 보고 3.00 m를 쓴다.
LANE_WIDTH_M = 3.0
LANE_WIDTH_FAST_M = 3.25  # motorway·trunk (설계속도 80 이상으로 가정)
# 같은 규칙 제16조 ③: 보도 유효폭 최소 2 m.
FOOTWAY_WIDTH_M = 2.0

# lanes 태그가 없을 때 차로 수 (가정: 유형별 흔한 값)
DEFAULT_LANES = {
    "motorway": 4, "trunk": 4, "primary": 4, "secondary": 2, "tertiary": 2,
    "motorway_link": 1, "trunk_link": 1, "primary_link": 1, "secondary_link": 1, "tertiary_link": 1,
    "unclassified": 2, "residential": 2, "living_street": 1, "service": 1, "busway": 1, "road": 2,
}
# 차로가 아닌 길의 전체 폭 (m)
PATH_WIDTH_M = {
    "footway": FOOTWAY_WIDTH_M,   # 규칙 제16조 보도 최소 유효폭
    "pedestrian": 4.0,            # 가정: 보행자 전용 거리
    "steps": FOOTWAY_WIDTH_M,     # 가정: 보도와 같게
    "cycleway": 1.5,              # 가정
    "path": 1.5,                  # 가정 (비포장일 수 있으나 포장으로 셈)
    "track": 3.0,                 # 가정
}
# 불투수로 세지 않는 highway 값 (실내 통로, 계획·공사 중)
SKIP_HIGHWAY = {"corridor", "proposed", "construction", "elevator", "platform", "bus_stop"}

TREE_RADIUS_M = 3.0          # 가정: natural=tree 점 하나의 수관 반경
TREE_ROW_WIDTH_M = 6.0       # 가정: natural=tree_row 수관 폭
# 하천 선의 수면 폭 (가정). 수면 폴리곤(natural=water 등)이 있으면 그쪽이 우선 반영된다.
WATERWAY_WIDTH_M = {"river": 20.0, "stream": 8.0, "canal": 8.0, "drain": 2.0, "ditch": 1.5}

GREEN = {
    "natural": {"wood", "tree_row", "scrub"},
    "landuse": {"forest", "grass", "meadow", "park", "village_green", "recreation_ground"},
    "leisure": {"park", "garden"},
}
PAVED_AMENITY = {"parking", "fuel", "marketplace", "bus_station"}


def road_width(tags: dict) -> float | None:
    hw = tags.get("highway")
    if hw is None or hw in SKIP_HIGHWAY:
        return None
    w = parse_number(tags.get("width"))
    if w and 0 < w < 60:
        return float(w)
    if hw in PATH_WIDTH_M:
        return PATH_WIDTH_M[hw]
    lanes = parse_number(tags.get("lanes"))
    n = int(lanes) if lanes and lanes > 0 else DEFAULT_LANES.get(hw, 1)
    lane = LANE_WIDTH_FAST_M if hw.split("_")[0] in ("motorway", "trunk") else LANE_WIDTH_M
    return n * lane


def _is_green(tags: dict) -> bool:
    return any(tags.get(k) in v for k, v in GREEN.items())


def _is_water_area(tags: dict) -> bool:
    return (
        tags.get("natural") == "water"
        or tags.get("waterway") in ("riverbank", "dock")
        or tags.get("landuse") in ("reservoir", "basin")
    )


def _is_paved_area(tags: dict) -> bool:
    return tags.get("amenity") in PAVED_AMENITY or ("highway" in tags and tags.get("area") == "yes")


@dataclass
class Layers:
    impervious: list[BaseGeometry]
    canopy: list[BaseGeometry]
    water: list[BaseGeometry]


def collect(osm: OsmData, proj: LocalProjection) -> Layers:
    imp, green, water = [], [], []

    for way in osm.ways.values():
        tags = way.get("tags") or {}
        if not tags:
            continue
        pts = osm.way_coords(way, proj)
        if not pts or len(pts) < 2:
            continue
        closed = len(pts) >= 4 and pts[0] == pts[-1]
        poly = Polygon(pts).buffer(0) if closed else None

        if "building" in tags and poly is not None:
            imp.append(poly)
        if _is_paved_area(tags):
            if poly is not None:
                imp.append(poly)
        elif "highway" in tags:
            w = road_width(tags)
            if w:
                imp.append(LineString(pts).buffer(w / 2, cap_style="flat"))
        if tags.get("natural") == "tree_row":
            green.append(LineString(pts).buffer(TREE_ROW_WIDTH_M / 2))
        elif _is_green(tags) and poly is not None:
            green.append(poly)
        if _is_water_area(tags) and poly is not None:
            water.append(poly)
        elif tags.get("waterway") in WATERWAY_WIDTH_M and tags.get("tunnel") not in ("yes", "culvert"):
            water.append(LineString(pts).buffer(WATERWAY_WIDTH_M[tags["waterway"]] / 2))

    for rel in osm.relations.values():
        tags = rel.get("tags") or {}
        targets = []
        if "building" in tags or _is_paved_area(tags):
            targets.append(imp)
        if _is_green(tags):
            targets.append(green)
        if _is_water_area(tags):
            targets.append(water)
        if targets:
            polys = [p.buffer(0) for p in _relation_polygons(osm, rel, proj)]
            for t in targets:
                t.extend(polys)

    for node in osm.tagged_nodes.values():
        if (node.get("tags") or {}).get("natural") == "tree":
            green.append(Point(proj.to_local(node["lat"], node["lon"])).buffer(TREE_RADIUS_M))

    return Layers(imp, green, water)


def coverage(geoms: list[BaseGeometry], width_m: float, height_m: float, resolution_m: float) -> np.ndarray:
    """도형 합집합이 각 셀 면적에서 차지하는 비율 (행 0 = 북쪽). 겹친 부분은 한 번만 센다."""
    rows, cols = grid_shape(width_m, height_m, resolution_m)
    out = np.zeros((rows, cols), dtype=np.float64)
    geoms = [g for g in geoms if not g.is_empty]
    if not geoms:
        return out
    area = box(-width_m / 2, -height_m / 2, width_m / 2, height_m / 2)
    union = unary_union(geoms).intersection(area)
    parts = [g for g in getattr(union, "geoms", [union]) if not g.is_empty]
    tree = STRtree(parts)
    for r in range(rows):
        y1 = height_m / 2 - r * resolution_m
        for c in range(cols):
            x0 = -width_m / 2 + c * resolution_m
            cell = box(x0, y1 - resolution_m, x0 + resolution_m, y1)
            hits = tree.query(cell, predicate="intersects")
            if len(hits):
                a = sum(parts[i].intersection(cell).area for i in hits)
                out[r, c] = min(1.0, a / cell.area)
    return out


def to_rgb(impervious: np.ndarray, canopy: np.ndarray, water: np.ndarray) -> np.ndarray:
    stack = np.stack([impervious, canopy, water], axis=-1)
    return np.clip(np.rint(stack * 255), 0, 255).astype(np.uint8)


def write_png_rgb8(path: Path, rgb: np.ndarray) -> None:
    import struct
    import zlib

    rows, cols, _ = rgb.shape
    raw = b"".join(b"\x00" + rgb[r].tobytes() for r in range(rows))

    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    ihdr = struct.pack(">IIBBBBB", cols, rows, 8, 2, 0, 0, 0)
    Path(path).write_bytes(
        b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    )


def read_png_rgb8(path: Path) -> np.ndarray:
    import struct
    import zlib

    data = Path(path).read_bytes()
    pos, idat, width = 8, b"", 0
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        kind, body = data[pos + 4 : pos + 8], data[pos + 8 : pos + 8 + length]
        if kind == b"IHDR":
            width, _, depth, color = struct.unpack(">IIBB", body[:10])
            assert depth == 8 and color == 2
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    raw = zlib.decompress(idat)
    stride = 1 + width * 3
    rows = [np.frombuffer(raw[i * stride + 1 : (i + 1) * stride], dtype=np.uint8).reshape(width, 3)
            for i in range(len(raw) // stride)]
    return np.stack(rows)


def build_landcover(
    out: Path, osm: OsmData, proj: LocalProjection, width_m: float, height_m: float, resolution_m: float = 30
) -> dict[str, np.ndarray]:
    layers = collect(osm, proj)
    grids = {
        "impervious": coverage(layers.impervious, width_m, height_m, resolution_m),
        "canopy": coverage(layers.canopy, width_m, height_m, resolution_m),
        "water": coverage(layers.water, width_m, height_m, resolution_m),
    }
    write_png_rgb8(Path(out) / "landcover.png", to_rgb(grids["impervious"], grids["canopy"], grids["water"]))
    return grids


def landcover_info(resolution_m: float = 30) -> dict:
    """manifest.json의 `landcover` 블록."""
    return {
        "file": "landcover.png",
        "resolution_m": resolution_m,
        "channels": {"r": "impervious", "g": "canopy", "b": "water"},
        "scale": 255,
    }
