"""가짜 도형으로 토지 피복 비율·래스터화·채널 매핑을 검사한다 (네트워크 없음)."""

import numpy as np
import pytest
from shapely.geometry import box

from dongne_gis import landcover as lc
from dongne_gis.osm import OsmData
from dongne_gis.project import LocalProjection

ORIGIN = (36.993, 127.087)
SIZE = 90  # ±45m → 30m 셀 3×3
P = LocalProjection(*ORIGIN)


def test_coverage_counts_overlap_once_and_north_up():
    # 북서쪽 셀(행 0, 열 0) 전체 + 같은 영역 겹침 + 가운데 셀 절반
    geoms = [box(-45, 15, -15, 45), box(-45, 15, -15, 45), box(-15, -15, 0, 15)]
    g = lc.coverage(geoms, SIZE, SIZE, 30)
    assert g.shape == (3, 3)
    assert g[0, 0] == pytest.approx(1.0)
    assert g[1, 1] == pytest.approx(0.5)
    assert g.sum() == pytest.approx(1.5)


def test_coverage_clips_to_area():
    g = lc.coverage([box(30, -45, 500, 45)], SIZE, SIZE, 30)  # 동쪽 열의 절반부터 밖으로
    assert g[:, 2].tolist() == pytest.approx([0.5, 0.5, 0.5])
    assert g[:, :2].sum() == 0


def test_road_width_rules():
    assert lc.road_width({"highway": "residential"}) == 2 * lc.LANE_WIDTH_M
    assert lc.road_width({"highway": "secondary", "lanes": "4"}) == 4 * lc.LANE_WIDTH_M
    assert lc.road_width({"highway": "trunk"}) == 4 * lc.LANE_WIDTH_FAST_M
    assert lc.road_width({"highway": "footway"}) == lc.FOOTWAY_WIDTH_M
    assert lc.road_width({"highway": "service", "width": "5"}) == 5
    assert lc.road_width({"highway": "corridor"}) is None


def node(nid, x, y, tags=None):
    lat, lon = P.to_latlon(x, y)
    n = {"type": "node", "id": nid, "lat": lat, "lon": lon}
    if tags:
        n["tags"] = tags
    return n


def test_collect_and_build(tmp_path):
    els = [
        node(1, -45, 0), node(2, 45, 0),                                  # 도로 (가로 90m)
        node(3, -45, 15), node(4, -15, 15), node(5, -15, 45), node(6, -45, 45),  # 공원 = 북서쪽 셀
        node(7, 15, -45), node(8, 15, 45),                                # 하천 선 (세로)
        node(9, 30, 30, {"natural": "tree"}),                             # 나무 한 그루
        node(10, -45, -45), node(11, -15, -45), node(12, -15, -15), node(13, -45, -15),  # 주차장
        {"type": "way", "id": 100, "nodes": [1, 2], "tags": {"highway": "residential", "lanes": "1"}},
        {"type": "way", "id": 101, "nodes": [3, 4, 5, 6, 3], "tags": {"leisure": "park"}},
        {"type": "way", "id": 102, "nodes": [7, 8], "tags": {"waterway": "stream"}},
        {"type": "way", "id": 103, "nodes": [10, 11, 12, 13, 10], "tags": {"amenity": "parking"}},
    ]
    osm = OsmData.from_overpass({"elements": els})
    grids = lc.build_landcover(tmp_path, osm, P, SIZE, SIZE)

    imp, can, wat = grids["impervious"], grids["canopy"], grids["water"]
    # 도로 폭 3m × 길이 90m = 270㎡, 주차장 900㎡
    assert imp.sum() * 900 == pytest.approx(270 + 900, rel=1e-6)
    assert imp[2, 0] == pytest.approx(1.0)                 # 남서쪽 셀 = 주차장
    assert imp[1, 1] == pytest.approx(3 * 30 / 900)       # 가운데 셀을 도로가 가로지름
    assert can[0, 0] == pytest.approx(1.0)
    assert can[0, 2] * 900 == pytest.approx(np.pi * lc.TREE_RADIUS_M**2, rel=0.01)
    width = lc.WATERWAY_WIDTH_M["stream"]
    assert wat[:, 2].sum() * 900 == pytest.approx(width / 2 * 90)  # 선 x=15 기준 동쪽 절반
    assert wat.sum() * 900 == pytest.approx(width * 90)

    rgb = lc.read_png_rgb8(tmp_path / "landcover.png")
    assert rgb.shape == (3, 3, 3) and rgb.dtype == np.uint8
    assert rgb[2, 0].tolist() == [255, 0, 0]
    assert rgb[0, 0].tolist() == [0, 255, 0]
    assert (rgb == lc.to_rgb(imp, can, wat)).all()


def test_to_rgb_scaling():
    a = np.array([[0.0, 0.5, 1.0]])
    rgb = lc.to_rgb(a, a * 0, np.ones_like(a) * 2)  # 범위 밖은 255로 자름
    assert rgb[0, :, 0].tolist() == [0, 128, 255]
    assert rgb[0, :, 2].tolist() == [255, 255, 255]


def test_manifest_block():
    info = lc.landcover_info()
    assert info["file"] == "landcover.png"
    assert info["channels"] == {"r": "impervious", "g": "canopy", "b": "water"}
