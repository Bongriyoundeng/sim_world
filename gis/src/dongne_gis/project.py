"""위경도 → origin 기준 로컬 미터 좌표 (x 동쪽, y 북쪽).

1.5km 정도의 좁은 범위라서 origin을 중심으로 하는 횡메르카토르(축척 1)를 씁니다.
origin에서 몇 km 안에서는 거리 왜곡이 무시할 만큼 작습니다.
"""

from __future__ import annotations

from pyproj import Transformer


class LocalProjection:
    def __init__(self, lat: float, lon: float) -> None:
        self.lat = lat
        self.lon = lon
        local = (
            f"+proj=tmerc +lat_0={lat} +lon_0={lon} +k=1 +x_0=0 +y_0=0 "
            "+ellps=WGS84 +units=m +no_defs"
        )
        self.proj4 = local
        self._fwd = Transformer.from_crs("EPSG:4326", local, always_xy=True)
        self._inv = Transformer.from_crs(local, "EPSG:4326", always_xy=True)

    def to_local(self, lat: float, lon: float) -> tuple[float, float]:
        x, y = self._fwd.transform(lon, lat)
        return x, y

    def to_latlon(self, x: float, y: float) -> tuple[float, float]:
        lon, lat = self._inv.transform(x, y)
        return lat, lon

    def bbox(self, width_m: float, height_m: float) -> tuple[float, float, float, float]:
        """origin 중심 width×height 사각형을 감싸는 (south, west, north, east)."""
        hw, hh = width_m / 2, height_m / 2
        corners = [self.to_latlon(x, y) for x in (-hw, hw) for y in (-hh, hh)]
        lats = [c[0] for c in corners]
        lons = [c[1] for c in corners]
        return min(lats), min(lons), max(lats), max(lons)
