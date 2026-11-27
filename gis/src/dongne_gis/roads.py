"""OSM highway way → roads.geojson (LineString, 로컬 미터)."""

from __future__ import annotations

from shapely.geometry import LineString, box

from .osm import OsmData, parse_number, r2
from .project import LocalProjection


def _oneway(tags: dict) -> tuple[bool, bool]:
    """(일방통행 여부, 좌표를 뒤집어야 하는지)."""
    v = tags.get("oneway", "").lower()
    if v == "-1":
        return True, True
    if v in ("yes", "true", "1"):
        return True, False
    if v in ("no", "false", "0"):
        return False, False
    if tags.get("junction") in ("roundabout", "circular") or tags.get("highway") == "motorway":
        return True, False
    return False, False


def build_roads(osm: OsmData, proj: LocalProjection, width_m: float, height_m: float) -> list[dict]:
    clip = box(-width_m / 2, -height_m / 2, width_m / 2, height_m / 2)
    features = []
    for wid in sorted(osm.ways):
        way = osm.ways[wid]
        tags = way.get("tags") or {}
        if "highway" not in tags or tags.get("area") == "yes":
            continue
        pts = osm.way_coords(way, proj)
        if not pts or len(pts) < 2:
            continue
        oneway, reverse = _oneway(tags)
        if reverse:
            pts.reverse()
        line = LineString(pts)
        if not line.intersects(clip):
            continue
        clipped = line.intersection(clip)
        parts = getattr(clipped, "geoms", [clipped])
        lanes = parse_number(tags.get("lanes"))
        props = {
            "osm_id": wid,
            "highway": tags["highway"],
            "name": tags.get("name"),
            "lanes": int(lanes) if lanes is not None else None,
            "oneway": oneway,
        }
        for part in parts:
            if part.geom_type != "LineString" or part.length == 0:
                continue
            coords = [[r2(x), r2(y)] for x, y in part.coords]
            features.append(
                {
                    "type": "Feature",
                    "properties": dict(props),
                    "geometry": {"type": "LineString", "coordinates": coords},
                }
            )
    return features
