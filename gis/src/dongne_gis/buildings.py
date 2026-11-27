"""OSM building way/relation → buildings.geojson (Polygon, 로컬 미터).

건물은 자르지 않고, 무게중심이 구역 안에 있는 것만 넣습니다.
멀티폴리곤 relation은 바깥 고리마다 Polygon 하나로 나눕니다.
"""

from __future__ import annotations

from shapely.geometry import LineString, Point, Polygon, box
from shapely.geometry.polygon import orient
from shapely.ops import polygonize, unary_union

from .osm import OsmData, parse_number, r2
from .project import LocalProjection


def _props(osm_id: int, tags: dict, osm_type: str) -> dict:
    return {
        "osm_id": osm_id,
        "osm_type": osm_type,  # way와 relation은 id 공간이 따로라서 함께 적는다
        "building": tags.get("building"),
        "building:levels": parse_number(tags.get("building:levels")),
        "height": parse_number(tags.get("height")),
    }


def _ring_polys(lines: list[list[tuple[float, float]]]) -> list[Polygon]:
    segs = [LineString(p) for p in lines if len(p) >= 2]
    return [p for p in polygonize(unary_union(segs)) if p.area > 0] if segs else []


def _relation_polygons(osm: OsmData, rel: dict, proj: LocalProjection) -> list[Polygon]:
    outers, inners = [], []
    for m in rel.get("members", []):
        if m.get("type") != "way" or m["ref"] not in osm.ways:
            continue
        pts = osm.way_coords(osm.ways[m["ref"]], proj)
        if pts:
            (inners if m.get("role") == "inner" else outers).append(pts)
    holes = _ring_polys(inners)
    result = []
    for outer in _ring_polys(outers):
        if any(outer.equals(h) for h in holes):
            continue
        inside = [h for h in holes if outer.contains(h)]
        result.append(Polygon(outer.exterior, [h.exterior for h in inside]))
    return result


def _feature(poly: Polygon, props: dict) -> dict:
    poly = orient(poly, sign=1.0)  # 바깥 고리 반시계 (GeoJSON RFC 7946)
    rings = [poly.exterior] + list(poly.interiors)
    coords = [[[r2(x), r2(y)] for x, y in ring.coords] for ring in rings]
    return {
        "type": "Feature",
        "properties": dict(props),
        "geometry": {"type": "Polygon", "coordinates": coords},
    }


def build_buildings(osm: OsmData, proj: LocalProjection, width_m: float, height_m: float) -> list[dict]:
    area = box(-width_m / 2, -height_m / 2, width_m / 2, height_m / 2)
    out: list[tuple[tuple[int, int], dict]] = []

    for wid, way in osm.ways.items():
        tags = way.get("tags") or {}
        if "building" not in tags:
            continue
        pts = osm.way_coords(way, proj)
        if not pts or len(pts) < 4 or pts[0] != pts[-1]:
            continue
        poly = Polygon(pts)
        if not poly.is_valid or poly.area == 0:
            continue
        if area.contains(Point(poly.centroid)):
            out.append(((0, wid), _feature(poly, _props(wid, tags, "way"))))

    for rid, rel in osm.relations.items():
        tags = rel.get("tags") or {}
        if "building" not in tags:
            continue
        for poly in _relation_polygons(osm, rel, proj):
            if area.contains(poly.centroid):
                out.append(((1, rid), _feature(poly, _props(rid, tags, "relation"))))

    out.sort(key=lambda kv: kv[0])
    return [f for _, f in out]
