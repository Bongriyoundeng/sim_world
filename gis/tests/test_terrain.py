"""작은 가짜 래스터로 지형 재표본·16비트 매핑을 검사한다 (네트워크 없음)."""

import numpy as np
import pytest
import rasterio
from rasterio.transform import from_origin

from dongne_gis import terrain
from dongne_gis.project import LocalProjection

ORIGIN = (36.993, 127.087)
SIZE = 300  # ±150m → 30m 셀 10×10


def fake_dem(path, func):
    """origin 주변 0.02° 범위, 1/3600° 간격(약 30m) EPSG:4326 래스터. 고도 = func(lat, lon)."""
    step = 1 / 3600
    west, north = ORIGIN[1] - 0.01, ORIGIN[0] + 0.01
    n = int(0.02 / step)
    lats = north - (np.arange(n) + 0.5) * step
    lons = west + (np.arange(n) + 0.5) * step
    data = func(lats[:, None], lons[None, :]).astype(np.float32)
    with rasterio.open(
        path, "w", driver="GTiff", height=n, width=n, count=1, dtype="float32",
        crs="EPSG:4326", transform=from_origin(west, north, step, step), nodata=-32767,
    ) as dst:
        dst.write(data, 1)
    return path


def test_tile_names():
    assert terrain.tile_name(36, 127) == "Copernicus_DSM_COG_10_N36_00_E127_00_DEM"
    assert terrain.tile_name(-1, -5) == "Copernicus_DSM_COG_10_S01_00_W005_00_DEM"
    assert terrain.tile_url("X").endswith("/X/X.tif")
    assert terrain.tiles_for_bbox((36.98, 127.07, 37.01, 127.10)) == [
        "Copernicus_DSM_COG_10_N36_00_E127_00_DEM", "Copernicus_DSM_COG_10_N37_00_E127_00_DEM"]


def test_resample_north_up_and_values(tmp_path):
    # 고도가 북쪽으로 갈수록 높아지는 경사면: 1m / 0.0001° (약 11m)
    src = fake_dem(tmp_path / "dem.tif", lambda la, lo: 20 + (la - ORIGIN[0]) * 10000 + 0 * lo)
    h = terrain.resample([src], *ORIGIN, SIZE, SIZE, 30)
    assert h.shape == (10, 10)
    assert (np.diff(h[:, 5]) < 0).all()  # 행 0 = 북쪽이 가장 높음
    p = LocalProjection(*ORIGIN)
    for r, c in [(0, 0), (4, 7), (9, 9)]:
        x, y = -SIZE / 2 + (c + 0.5) * 30, SIZE / 2 - (r + 0.5) * 30
        lat, _ = p.to_latlon(x, y)
        assert h[r, c] == pytest.approx(20 + (lat - ORIGIN[0]) * 10000, abs=0.05)


def test_uint16_mapping_and_range():
    h = np.array([[2.931, 10.0], [47.115, 25.5]], dtype=np.float32)
    lo, hi = terrain.elevation_range(h)
    assert (lo, hi) == (2.93, 47.12)
    px = terrain.to_uint16(h, lo, hi)
    assert px.dtype == np.uint16 and px.min() >= 0 and px.max() <= 65535
    back = terrain.from_uint16(px, lo, hi)
    assert np.abs(back - h).max() < (hi - lo) / 65535
    assert terrain.to_uint16(np.full((2, 2), 5.0), 5.0, 5.0).tolist() == [[0, 0], [0, 0]]


def test_png16_round_trip(tmp_path):
    px = (np.arange(12, dtype=np.uint16) * 5000).reshape(3, 4)
    terrain.write_png16(tmp_path / "t.png", px)
    assert (terrain.read_png16(tmp_path / "t.png") == px).all()


def test_build_terrain_offline(tmp_path, monkeypatch):
    src = fake_dem(tmp_path / "dem.tif", lambda la, lo: 5 + (lo - ORIGIN[1]) * 5000 + 0 * la)
    monkeypatch.setattr(terrain, "fetch_tile", lambda name, cache_dir=None: src)
    heights, lo, hi = terrain.build_terrain(tmp_path, *ORIGIN, SIZE, SIZE)
    assert lo <= heights.min() and heights.max() <= hi
    px = terrain.read_png16(tmp_path / "terrain.png")
    assert px.shape == (10, 10)
    assert px[:, 0].mean() < px[:, -1].mean()  # 열 끝 = 동쪽이 높음
    # min/max를 소수 2자리 바깥쪽으로 반올림하므로 양 끝은 0·65535에 아주 가깝다
    tol = 0.01 / (hi - lo) * 65535 + 1
    assert px.min() <= tol and px.max() >= 65535 - tol
    assert np.abs(terrain.from_uint16(px, lo, hi) - heights).max() < (hi - lo) / 65535


def test_cells_on_lines():
    cells = terrain.cells_on_lines([[(-150, 135), (150, 135)]], SIZE, SIZE, 30)
    assert cells == {(0, c) for c in range(10)}
