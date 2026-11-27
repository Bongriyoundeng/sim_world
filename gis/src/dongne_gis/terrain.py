"""Copernicus DEM GLO-30 → terrain.png (16비트 그레이스케일 높이맵).

- 원본: AWS Open Data 버킷 `copernicus-dem-30m` (Cloud Optimized GeoTIFF, 1°×1° 타일).
  근거: docs/design/research-log.md#copernicus-dem-aws
- 격자: manifest와 같은 origin 중심 로컬 횡메르카토르, 셀 크기 resolution_m.
  행 0 = 북쪽 가장자리, 열 0 = 서쪽 가장자리. 값은 셀 중심 고도(쌍선형 보간).
- 픽셀 0~65535는 min_elevation_m~max_elevation_m에 선형 대응 (schema/map-package/README.md).
"""

from __future__ import annotations

import math
import struct
import urllib.request
import zlib
from pathlib import Path

import numpy as np
import rasterio
from rasterio.crs import CRS
from rasterio.transform import from_origin
from rasterio.warp import Resampling, reproject

from .fetch import DEFAULT_CACHE, USER_AGENT
from .project import LocalProjection

DEM_BUCKET = "https://copernicus-dem-30m.s3.amazonaws.com"
NODATA = -32767.0


def tile_name(lat_floor: int, lon_floor: int) -> str:
    """1°×1° 타일 이름. 남서쪽 모서리의 정수 위경도로 짓는다 (예: N36_00_E127_00)."""
    ns = f"{'N' if lat_floor >= 0 else 'S'}{abs(lat_floor):02d}_00"
    ew = f"{'E' if lon_floor >= 0 else 'W'}{abs(lon_floor):03d}_00"
    return f"Copernicus_DSM_COG_10_{ns}_{ew}_DEM"


def tile_url(name: str) -> str:
    return f"{DEM_BUCKET}/{name}/{name}.tif"


def tiles_for_bbox(bbox: tuple[float, float, float, float]) -> list[str]:
    s, w, n, e = bbox
    return [
        tile_name(la, lo)
        for la in range(math.floor(s), math.floor(n) + 1)
        for lo in range(math.floor(w), math.floor(e) + 1)
    ]


def fetch_tile(name: str, cache_dir: Path | None = None) -> Path:
    cache_dir = Path(cache_dir) if cache_dir else DEFAULT_CACHE
    path = cache_dir / "copernicus-dem-30m" / f"{name}.tif"
    if path.exists():
        return path
    path.parent.mkdir(parents=True, exist_ok=True)
    req = urllib.request.Request(tile_url(name), headers={"User-Agent": USER_AGENT})
    tmp = path.with_suffix(".part")
    with urllib.request.urlopen(req, timeout=600) as resp, open(tmp, "wb") as f:
        while chunk := resp.read(1 << 20):
            f.write(chunk)
    tmp.replace(path)
    return path


def local_crs(lat: float, lon: float) -> CRS:
    return CRS.from_proj4(LocalProjection(lat, lon).proj4)


def grid_shape(width_m: float, height_m: float, resolution_m: float) -> tuple[int, int]:
    return math.ceil(height_m / resolution_m), math.ceil(width_m / resolution_m)


def resample(
    sources: list[Path],
    lat: float,
    lon: float,
    width_m: float,
    height_m: float,
    resolution_m: float = 30,
) -> np.ndarray:
    """원본 래스터들을 로컬 격자(행 0 = 북쪽)로 재표본화한 float32 배열 (행, 열)."""
    rows, cols = grid_shape(width_m, height_m, resolution_m)
    dst = np.full((rows, cols), np.nan, dtype=np.float32)
    transform = from_origin(-width_m / 2, height_m / 2, resolution_m, resolution_m)
    crs = local_crs(lat, lon)
    for src_path in sources:
        with rasterio.open(src_path) as src:
            part = np.full((rows, cols), np.nan, dtype=np.float32)
            reproject(
                source=rasterio.band(src, 1),
                destination=part,
                dst_transform=transform,
                dst_crs=crs,
                dst_nodata=np.nan,
                src_nodata=src.nodata if src.nodata is not None else NODATA,
                resampling=Resampling.bilinear,
            )
            fill = np.isnan(dst) & ~np.isnan(part)
            dst[fill] = part[fill]
    if np.isnan(dst).any():
        # 바다처럼 타일이 없는 곳은 0m (버킷 안내: 바다 타일은 없음, 고도 0으로 간주)
        dst = np.nan_to_num(dst, nan=0.0)
    return dst


def elevation_range(heights: np.ndarray) -> tuple[float, float]:
    """manifest에 적을 범위. 소수 2자리로 내림/올림해 모든 값이 범위 안에 들게 한다."""
    lo = math.floor(float(heights.min()) * 100) / 100
    hi = math.ceil(float(heights.max()) * 100) / 100
    return lo, hi


def to_uint16(heights: np.ndarray, lo: float, hi: float) -> np.ndarray:
    if hi <= lo:
        return np.zeros(heights.shape, dtype=np.uint16)
    scaled = (heights.astype(np.float64) - lo) / (hi - lo) * 65535
    return np.clip(np.rint(scaled), 0, 65535).astype(np.uint16)


def from_uint16(pixels: np.ndarray, lo: float, hi: float) -> np.ndarray:
    return lo + pixels.astype(np.float64) / 65535 * (hi - lo)


def write_png16(path: Path, pixels: np.ndarray) -> None:
    """16비트 그레이스케일 PNG (외부 이미지 라이브러리 없이)."""
    rows, cols = pixels.shape
    raw = b"".join(b"\x00" + pixels[r].astype(">u2").tobytes() for r in range(rows))

    def chunk(kind: bytes, data: bytes) -> bytes:
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    ihdr = struct.pack(">IIBBBBB", cols, rows, 16, 0, 0, 0, 0)
    Path(path).write_bytes(
        b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
    )


def read_png16(path: Path) -> np.ndarray:
    """write_png16이 쓴 형식(필터 0, 단일 IDAT 포함 일반 IDAT 연결)을 읽는다. 테스트·검증용."""
    data = Path(path).read_bytes()
    assert data[:8] == b"\x89PNG\r\n\x1a\n"
    pos, idat, width = 8, b"", 0
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        kind = data[pos + 4 : pos + 8]
        body = data[pos + 8 : pos + 8 + length]
        if kind == b"IHDR":
            width, height, depth, color = struct.unpack(">IIBB", body[:10])
            assert depth == 16 and color == 0
        elif kind == b"IDAT":
            idat += body
        pos += 12 + length
    raw = zlib.decompress(idat)
    stride = 1 + width * 2
    rows = [np.frombuffer(raw[i * stride + 1 : (i + 1) * stride], dtype=">u2") for i in range(len(raw) // stride)]
    return np.vstack(rows).astype(np.uint16)


def build_terrain(
    out: Path,
    lat: float,
    lon: float,
    width_m: float,
    height_m: float,
    resolution_m: float = 30,
    cache_dir: Path | None = None,
) -> tuple[np.ndarray, float, float]:
    proj = LocalProjection(lat, lon)
    tiles = [fetch_tile(t, cache_dir) for t in tiles_for_bbox(proj.bbox(width_m, height_m))]
    heights = resample(tiles, lat, lon, width_m, height_m, resolution_m)
    lo, hi = elevation_range(heights)
    write_png16(Path(out) / "terrain.png", to_uint16(heights, lo, hi))
    return heights, lo, hi


def cells_on_lines(
    lines: list[list[tuple[float, float]]], width_m: float, height_m: float, resolution_m: float
) -> set[tuple[int, int]]:
    """로컬 미터 선들이 지나는 격자 셀 (행, 열). 짧은 간격으로 표본을 찍어 근사한다."""
    rows, cols = grid_shape(width_m, height_m, resolution_m)
    cells = set()
    step = resolution_m / 4
    for pts in lines:
        for (x0, y0), (x1, y1) in zip(pts, pts[1:]):
            n = max(1, int(math.hypot(x1 - x0, y1 - y0) / step))
            for i in range(n + 1):
                x = x0 + (x1 - x0) * i / n
                y = y0 + (y1 - y0) * i / n
                c = math.floor((x + width_m / 2) / resolution_m)
                r = math.floor((height_m / 2 - y) / resolution_m)
                if 0 <= r < rows and 0 <= c < cols:
                    cells.add((r, c))
    return cells
