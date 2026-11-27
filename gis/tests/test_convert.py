"""작은 가짜 Overpass 응답으로 도로·건물 변환을 검사한다 (네트워크 없음)."""

import json

import pytest
from shapely.geometry import Polygon

from dongne_gis import package
from dongne_gis.buildings import build_buildings
from dongne_gis.landmarks import parse_landmarks
from dongne_gis.osm import OsmData, parse_number
from dongne_gis.project import LocalProjection
from dongne_gis.roads import build_roads

ORIGIN = (36.993, 127.087)
SIZE = 200  # ±100m
P = LocalProjection(*ORIGIN)


def node(nid, x, y):
    lat, lon = P.to_latlon(x, y)
    return {"type": "node", "id": nid, "lat": lat, "lon": lon}


def fake_response():
    pts = {
        1: (-50, 0), 2: (50, 0),            # 도로 A (안)
        3: (-50, 20), 4: (300, 20),         # 도로 B (밖으로 나감 → 자름)
        5: (10, 10), 6: (30, 10), 7: (30, 30), 8: (10, 30),  # 건물 way
        9: (500, 500), 10: (510, 500), 11: (510, 510),      # 밖의 건물
        20: (-80, -80), 21: (-40, -80), 22: (-40, -40),     # 멀티폴리곤 바깥 (두 way)
        23: (-80, -40),
        30: (-70, -70), 31: (-50, -70), 32: (-50, -50), 33: (-70, -50),  # 안쪽 고리
    }
    els = [node(i, x, y) for i, (x, y) in pts.items()]
    els += [
        {"type": "way", "id": 100, "nodes": [1, 2],
         "tags": {"highway": "residential", "name": "화촌길", "lanes": "2"}},
        {"type": "way", "id": 101, "nodes": [3, 4],
         "tags": {"highway": "secondary", "oneway": "-1", "lanes": "3;2"}},
        {"type": "way", "id": 102, "nodes": [5, 6, 7, 8, 5],
         "tags": {"highway": "pedestrian", "area": "yes"}},
        {"type": "way", "id": 200, "nodes": [5, 6, 7, 8, 5],
         "tags": {"building": "apartments", "building:levels": "15", "height": "45 m"}},
        {"type": "way", "id": 201, "nodes": [9, 10, 11, 9], "tags": {"building": "yes"}},
        {"type": "way", "id": 300, "nodes": [20, 21, 22]},
        {"type": "way", "id": 301, "nodes": [22, 23, 20]},
        {"type": "way", "id": 302, "nodes": [30, 31, 32, 33, 30]},
        {"type": "relation", "id": 400, "tags": {"type": "multipolygon", "building": "school"},
         "members": [{"type": "way", "ref": 300, "role": "outer"},
                     {"type": "way", "ref": 301, "role": "outer"},
                     {"type": "way", "ref": 302, "role": "inner"}]},
    ]
    return {"osm3s": {"timestamp_osm_base": "2026-01-01T00:00:00Z"}, "elements": els}


@pytest.fixture
def osm():
    return OsmData.from_overpass(fake_response())


def test_roads(osm):
    roads = build_roads(osm, P, SIZE, SIZE)
    by_id = {f["properties"]["osm_id"]: f for f in roads}
    assert set(by_id) == {100, 101}  # area=yes 보행 광장은 빠짐

    a = by_id[100]
    assert a["geometry"]["type"] == "LineString"
    assert a["properties"] == {"osm_id": 100, "highway": "residential", "name": "화촌길",
                               "lanes": 2, "oneway": False}
    assert a["geometry"]["coordinates"] == [[-50.0, 0.0], [50.0, 0.0]]

    b = by_id[101]
    assert b["properties"]["oneway"] is True and b["properties"]["lanes"] == 3
    coords = b["geometry"]["coordinates"]
    # oneway=-1 → 진행 방향으로 뒤집고, 범위 경계 x=100에서 자름
    assert coords[0] == [100.0, 20.0] and coords[-1] == [-50.0, 20.0]
    assert all(len(str(v).split(".")[-1]) <= 2 for c in coords for v in c)


def test_buildings(osm):
    blds = build_buildings(osm, P, SIZE, SIZE)
    assert [(f["properties"]["osm_type"], f["properties"]["osm_id"]) for f in blds] == [
        ("way", 200), ("relation", 400)]

    apt = blds[0]
    assert apt["properties"]["building"] == "apartments"
    assert apt["properties"]["building:levels"] == 15
    assert apt["properties"]["height"] == 45
    ring = apt["geometry"]["coordinates"][0]
    assert ring[0] == ring[-1]
    assert Polygon(ring).area == pytest.approx(400, abs=0.5)
    assert Polygon(ring).exterior.is_ccw

    school = blds[1]
    assert school["properties"]["building:levels"] is None
    outer, inner = school["geometry"]["coordinates"]
    assert Polygon(outer, [inner]).area == pytest.approx(1600 - 400, abs=1)


def test_parse_number():
    assert parse_number("3") == 3
    assert parse_number("2.5") == 2.5
    assert parse_number("12 m") == 12
    assert parse_number("4;5") == 4
    assert parse_number("many") is None
    assert parse_number(None) is None


def test_build_package_offline(tmp_path, monkeypatch):
    monkeypatch.setattr(package, "overpass", lambda *a, **k: fake_response())
    out = tmp_path / "pkg"
    s = package.build_package("t", *ORIGIN, SIZE, out, "uv run python -m dongne_gis build ...", terrain=False)
    manifest = json.loads((out / "manifest.json").read_text(encoding="utf-8"))
    assert manifest["extent_m"] == {"width": 200, "height": 200}
    assert manifest["terrain"]["file"] == "terrain.png"
    assert any("OpenStreetMap" in a for a in manifest["attribution"])
    roads = json.loads((out / "roads.geojson").read_text(encoding="utf-8"))
    blds = json.loads((out / "buildings.geojson").read_text(encoding="utf-8"))
    assert len(roads["features"]) == 2 and len(blds["features"]) == 2
    assert s["buildings_with_levels"] == 1
    readme = (out / "README.md").read_text(encoding="utf-8")
    assert "ODbL" in readme and "© OpenStreetMap contributors" in readme


def test_landmarks():
    resp = {"elements": [
        {"type": "node", "id": 1, "tags": {"railway": "station", "name": "평택"},
         **{k: v for k, v in node(1, -150, -240).items() if k in ("lat", "lon")}},
        {"type": "way", "id": 2, "tags": {"waterway": "stream", "name": "통복천"},
         "geometry": [{"lat": P.to_latlon(x, 0)[0], "lon": P.to_latlon(x, 0)[1]} for x in (-2000, -1000)]},
    ]}
    lms = parse_landmarks(resp, P)
    assert lms[0].kind == "station" and lms[0].inside(1500, 1500)
    assert lms[1].kind == "stream" and not lms[1].inside(1500, 1500)
