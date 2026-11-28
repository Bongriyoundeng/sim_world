"""국토교통부 건축HUB 건축물대장 표제부 → 법정동 단위 건물 통계 (2단계 #36, ADR 0008).

- 서비스 키는 환경변수 DATA_GO_KR_KEY에서만 읽는다. 키·요청 URL은 출력하거나 저장하지 않는다.
- 캐시(data-raw/building-register/)에는 응답에서 고른 필드만 저장한다 (키가 들어간 URL 없음).
- 건축물대장에는 좌표가 없어 OSM 건물과 하나씩 맞출 수 없다. 대신 법정동 단위로 주용도별 연면적·층수·세대 수
  분포를 만들어, OSM 건물 중 용도를 모르는 건물의 가정값을 이 분포로 바꾼다.

    DATA_GO_KR_KEY=... uv run python -m dongne_gis register --sigungu 41220 --bjdong 11300 11400 11500

출처 표기: 국토교통부, 「건축HUB 건축물대장정보 서비스」, 공공데이터포털 (이용허락범위 제한 없음)
"""

from __future__ import annotations

import json
import os
import time
import urllib.error
import urllib.parse
import urllib.request
from collections import defaultdict
from pathlib import Path

from .fetch import DEFAULT_CACHE, USER_AGENT

BASE = "https://apis.data.go.kr/1613000/BldRgstHubService/getBrTitleInfo"
FIELDS = (
    "platPlc", "bun", "ji", "mainPurpsCdNm", "etcPurps", "grndFlrCnt", "ugrndFlrCnt",
    "totArea", "archArea", "platArea", "hhldCnt", "fmlyCnt", "hoCnt", "useAprDay",
)

# 건축물대장 주용도 → 시뮬레이션 용도 (land_use.rs의 BuildingUse와 같은 이름)
PURPOSE_TO_USE = {
    "단독주택": "residential",
    "공동주택": "residential",
    "제1종근린생활시설": "commercial",
    "제2종근린생활시설": "commercial",
    "판매시설": "commercial",
    "숙박시설": "commercial",
    "위락시설": "commercial",
    "업무시설": "office",
    "교육연구시설": "public",
    "노유자시설": "public",
    "의료시설": "public",
    "문화및집회시설": "public",
    "종교시설": "public",
    "운동시설": "public",
    "운수시설": "public",
    "공공용시설": "public",
    "공장": "industrial",
    "창고시설": "industrial",
    "위험물저장및처리시설": "industrial",
    "자동차관련시설": "industrial",
}


class RegisterError(RuntimeError):
    """API 오류 (키·승인 문제 등). 메시지에 키는 넣지 않는다."""


def _num(v) -> float:
    try:
        return float(v)
    except (TypeError, ValueError):
        return 0.0


def _request(key: str, params: dict) -> dict:
    q = urllib.parse.urlencode(params)
    url = f"{BASE}?serviceKey={key}&{q}"
    req = urllib.request.Request(url, headers={"User-Agent": USER_AGENT})
    for attempt in range(4):
        try:
            with urllib.request.urlopen(req, timeout=60) as r:
                body = r.read().decode("utf-8", "replace")
            break
        except urllib.error.HTTPError as e:
            if e.code in (429, 500, 502, 503, 504) and attempt < 3:
                time.sleep(5 * 2**attempt)
                continue
            raise RegisterError(f"HTTP {e.code}") from None
    try:
        d = json.loads(body)
    except ValueError:
        import re

        codes = re.findall(r"<(returnReasonCode|resultCode)>([^<]+)<", body)
        raise RegisterError(f"JSON이 아닌 응답 (코드 {codes[:2]})") from None
    header = d.get("response", {}).get("header", {})
    if header.get("resultCode") != "00":
        raise RegisterError(f"결과 코드 {header.get('resultCode')} {header.get('resultMsg')}")
    return d["response"].get("body", {})


def fetch_dong(sigungu: str, bjdong: str, cache_dir: Path | None = None, refresh: bool = False) -> list[dict]:
    """법정동 하나의 표제부 전체 (100건씩). 캐시가 있으면 그대로 쓴다."""
    cache_dir = Path(cache_dir) if cache_dir else DEFAULT_CACHE
    path = cache_dir / "building-register" / f"title_{sigungu}_{bjdong}.json"
    if path.exists() and not refresh:
        return json.loads(path.read_text(encoding="utf-8"))["items"]
    key = os.environ.get("DATA_GO_KR_KEY", "")
    if not key:
        raise RegisterError("환경변수 DATA_GO_KR_KEY가 없음")
    items: list[dict] = []
    page = 1
    while True:
        body = _request(key, {"sigunguCd": sigungu, "bjdongCd": bjdong, "numOfRows": "100",
                              "pageNo": str(page), "_type": "json"})
        got = (body.get("items") or {}).get("item") or []
        if isinstance(got, dict):
            got = [got]
        items += [{k: it.get(k) for k in FIELDS} for it in got]
        total = int(body.get("totalCount") or 0)
        if not got or len(items) >= total:
            break
        page += 1
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"sigungu": sigungu, "bjdong": bjdong, "fetched": time.strftime("%Y-%m-%d"),
                                "source": "국토교통부 건축HUB 건축물대장정보 서비스 getBrTitleInfo",
                                "items": items}, ensure_ascii=False), encoding="utf-8")
    return items


def summarize(items: list[dict]) -> dict:
    """주용도·시뮬레이션 용도별 건물 수, 연면적, 평균 지상층수, 세대 수."""
    by_purpose: dict[str, dict] = defaultdict(lambda: {"buildings": 0, "tot_area_m2": 0.0, "floors_sum": 0.0, "households": 0.0})
    by_use: dict[str, dict] = defaultdict(lambda: {"buildings": 0, "tot_area_m2": 0.0, "floors_sum": 0.0, "households": 0.0})
    floor_hist: dict[int, int] = defaultdict(int)
    for it in items:
        purpose = (it.get("mainPurpsCdNm") or "(없음)").strip() or "(없음)"
        use = PURPOSE_TO_USE.get(purpose, "other")
        floors = _num(it.get("grndFlrCnt"))
        area = _num(it.get("totArea"))
        hh = _num(it.get("hhldCnt")) + _num(it.get("fmlyCnt")) + _num(it.get("hoCnt"))
        for agg in (by_purpose[purpose], by_use[use]):
            agg["buildings"] += 1
            agg["tot_area_m2"] += area
            agg["floors_sum"] += floors
            agg["households"] += hh
        if floors > 0:
            floor_hist[min(int(floors), 20)] += 1
    for agg in list(by_purpose.values()) + list(by_use.values()):
        agg["mean_floors"] = round(agg["floors_sum"] / agg["buildings"], 2) if agg["buildings"] else 0.0
        del agg["floors_sum"]
        agg["tot_area_m2"] = round(agg["tot_area_m2"], 1)
    total_area = sum(a["tot_area_m2"] for a in by_use.values())
    res = by_use.get("residential", {}).get("tot_area_m2", 0.0)
    com = by_use.get("commercial", {}).get("tot_area_m2", 0.0)
    return {
        "records": len(items),
        "total_floor_area_m2": round(total_area, 1),
        "residential_share_of_res_and_com": round(res / (res + com), 3) if res + com else None,
        "by_use": dict(sorted(by_use.items())),
        "by_purpose": dict(sorted(by_purpose.items(), key=lambda kv: -kv[1]["tot_area_m2"])),
        "floor_histogram": dict(sorted(floor_hist.items())),
    }
