import math

import pytest

from dongne_gis.project import LocalProjection

ORIGIN = (36.993, 127.087)


def test_origin_is_zero():
    p = LocalProjection(*ORIGIN)
    x, y = p.to_local(*ORIGIN)
    assert abs(x) < 1e-6 and abs(y) < 1e-6


@pytest.mark.parametrize("x,y", [(0, 0), (750, 750), (-750, 750), (-750, -750), (123.45, -678.9), (2000, -1500)])
def test_round_trip_under_10cm(x, y):
    p = LocalProjection(*ORIGIN)
    lat, lon = p.to_latlon(x, y)
    x2, y2 = p.to_local(lat, lon)
    assert math.hypot(x2 - x, y2 - y) < 0.1


def test_axes_east_north():
    p = LocalProjection(*ORIGIN)
    x, y = p.to_local(ORIGIN[0] + 0.001, ORIGIN[1])  # 북쪽으로 약 111m
    assert abs(x) < 0.5 and 105 < y < 117
    x, y = p.to_local(ORIGIN[0], ORIGIN[1] + 0.001)  # 동쪽으로 약 89m
    assert 85 < x < 93 and abs(y) < 0.5


def test_bbox_contains_square():
    p = LocalProjection(*ORIGIN)
    s, w, n, e = p.bbox(1500, 1500)
    for x in (-750, 750):
        for y in (-750, 750):
            lat, lon = p.to_latlon(x, y)
            assert s - 1e-9 <= lat <= n + 1e-9 and w - 1e-9 <= lon <= e + 1e-9
