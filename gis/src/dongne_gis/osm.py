"""Overpass JSON 응답을 다루는 공통 도구."""

from __future__ import annotations

import re
from dataclasses import dataclass, field

from .project import LocalProjection


@dataclass
class OsmData:
    nodes: dict[int, tuple[float, float]] = field(default_factory=dict)  # id → (lat, lon)
    ways: dict[int, dict] = field(default_factory=dict)
    relations: dict[int, dict] = field(default_factory=dict)
    timestamp: str | None = None

    @classmethod
    def from_overpass(cls, resp: dict) -> "OsmData":
        d = cls(timestamp=(resp.get("osm3s") or {}).get("timestamp_osm_base"))
        for el in resp.get("elements", []):
            t = el.get("type")
            if t == "node":
                d.nodes[el["id"]] = (el["lat"], el["lon"])
            elif t == "way":
                # skel 출력(태그 없음)이 body 출력을 덮어쓰지 않게 한다
                if el["id"] not in d.ways or el.get("tags"):
                    d.ways[el["id"]] = el
            elif t == "relation":
                d.relations[el["id"]] = el
        return d

    def way_coords(self, way: dict, proj: LocalProjection) -> list[tuple[float, float]] | None:
        pts = []
        for nid in way.get("nodes", []):
            ll = self.nodes.get(nid)
            if ll is None:
                return None
            pts.append(proj.to_local(*ll))
        return pts


def r2(v: float) -> float:
    return round(v, 2)


def parse_number(value: str | None) -> float | None:
    """'3', '2.5', '12 m', '3;4' 같은 값에서 첫 숫자를 꺼낸다."""
    if value is None:
        return None
    m = re.search(r"-?\d+(?:\.\d+)?", value.replace(",", "."))
    if not m:
        return None
    n = float(m.group())
    return int(n) if n.is_integer() else n
