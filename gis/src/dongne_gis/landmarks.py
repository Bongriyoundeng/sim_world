"""구역 검증: 평택역과 통복천이 맵 범위 안에 들어가는지 확인한다."""

from __future__ import annotations

from dataclasses import dataclass

from .project import LocalProjection


@dataclass
class Landmark:
    kind: str  # "station" | "stream"
    osm_type: str
    osm_id: int
    name: str
    points: list[tuple[float, float]]  # 로컬 미터

    def inside(self, width_m: float, height_m: float) -> bool:
        """역은 대표점이, 하천은 꼭짓점 하나라도 범위 안에 있으면 포함으로 본다."""
        hw, hh = width_m / 2, height_m / 2
        return any(abs(x) <= hw and abs(y) <= hh for x, y in self.points)


def parse_landmarks(resp: dict, proj: LocalProjection) -> list[Landmark]:
    result = []
    for el in resp.get("elements", []):
        tags = el.get("tags") or {}
        kind = "station" if tags.get("railway") == "station" else "stream"
        if el["type"] == "node":
            pts = [proj.to_local(el["lat"], el["lon"])]
        elif "geometry" in el:
            pts = [proj.to_local(g["lat"], g["lon"]) for g in el["geometry"] if g]
        elif "bounds" in el:
            b = el["bounds"]
            pts = [proj.to_local((b["minlat"] + b["maxlat"]) / 2, (b["minlon"] + b["maxlon"]) / 2)]
        else:
            continue
        if kind == "station" and len(pts) > 1:
            pts = [(sum(p[0] for p in pts) / len(pts), sum(p[1] for p in pts) / len(pts))]
        result.append(Landmark(kind, el["type"], el["id"], tags.get("name", ""), pts))
    return result
