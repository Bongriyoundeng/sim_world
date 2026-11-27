"""Overpass API로 OSM 데이터를 받는다. 원본 응답은 data-raw/에 캐시한다 (커밋하지 않음)."""

from __future__ import annotations

import hashlib
import json
import time
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

OVERPASS_URL = "https://overpass-api.de/api/interpreter"
USER_AGENT = "sim_world-gis/0.1"
DEFAULT_CACHE = Path(__file__).resolve().parents[3] / "data-raw"

BBox = tuple[float, float, float, float]  # south, west, north, east


def _bbox_str(bbox: BBox) -> str:
    return ",".join(f"{v:.6f}" for v in bbox)


def map_query(bbox: BBox) -> str:
    """bbox 안의 highway way, building way/relation과 그 구성 요소를 한 번에 받는 쿼리."""
    b = _bbox_str(bbox)
    return (
        "[out:json][timeout:180];\n"
        f'(way["highway"]({b});way["building"]({b});relation["building"]({b}););\n'
        "out body;\n>;\nout skel qt;\n"
    )


def landmark_query(bbox: BBox, pad_deg: float = 0.03) -> str:
    """구역 검증용: bbox 주변의 평택역(railway=station)과 통복천(waterway)."""
    s, w, n, e = bbox
    b = _bbox_str((s - pad_deg, w - pad_deg, n + pad_deg, e + pad_deg))
    return (
        "[out:json][timeout:60];\n"
        f'(nwr["railway"="station"]["name"~"평택"]({b});'
        f'way["waterway"]["name"~"통복천"]({b}););\n'
        "out geom;\n"
    )


def _post_with_retry(req: urllib.request.Request, attempts: int = 4) -> bytes:
    """Overpass가 바쁠 때 내는 429/502/503/504는 잠시 기다렸다 다시 시도한다."""
    for i in range(attempts):
        try:
            with urllib.request.urlopen(req, timeout=300) as resp:
                return resp.read()
        except urllib.error.HTTPError as e:
            if e.code not in (429, 502, 503, 504) or i == attempts - 1:
                raise
            time.sleep(15 * 2**i)
    raise AssertionError("unreachable")


def overpass(query: str, cache_dir: Path | None = None, refresh: bool = False) -> dict:
    cache_dir = Path(cache_dir) if cache_dir else DEFAULT_CACHE
    key = hashlib.sha256(query.encode()).hexdigest()[:16]
    path = cache_dir / f"overpass-{key}.json"
    if path.exists() and not refresh:
        return json.loads(path.read_text(encoding="utf-8"))
    data = urllib.parse.urlencode({"data": query}).encode()
    req = urllib.request.Request(
        OVERPASS_URL, data=data, headers={"User-Agent": USER_AGENT}
    )
    raw = _post_with_retry(req)
    result = json.loads(raw)
    cache_dir.mkdir(parents=True, exist_ok=True)
    path.write_bytes(raw)
    return result
