"""기상청 종관기상관측(ASOS) 시간 자료 → 기후 사건 시계열 (2단계 #38, ADR 0008).

- 공공데이터포털 「기상청_지상(종관, ASOS) 시간자료 조회서비스」. 서비스 키는 DATA_GO_KR_KEY 환경변수에서만 읽고
  출력·저장하지 않는다. 이 API는 공공데이터포털에서 따로 활용 신청·승인이 필요하다(2026-11-28: HTTP 403).
- 캐시(data-raw/kma/)에는 고른 필드만 저장한다.
- 출처 표기(필수): 기상청 기상자료개방포털, 종관기상관측(ASOS) 자료

    DATA_GO_KR_KEY=... uv run python -m dongne_gis asos --station 119 --date 2024-07-18
"""

from __future__ import annotations

import json
import os
import urllib.error
import urllib.parse
import urllib.request
from pathlib import Path

from .fetch import DEFAULT_CACHE, USER_AGENT

BASE = "https://apis.data.go.kr/1360000/AsosHourlyInfoService/getWthrDataList"
SOURCE = "기상청 기상자료개방포털, 종관기상관측(ASOS) 자료"


class KmaError(RuntimeError):
    """API 오류. 메시지에 키를 넣지 않는다."""


def fetch_day(station: str, date: str, cache_dir: Path | None = None, refresh: bool = False) -> list[dict]:
    """하루(0~23시) 시간 자료. date는 YYYY-MM-DD."""
    cache_dir = Path(cache_dir) if cache_dir else DEFAULT_CACHE
    ymd = date.replace("-", "")
    path = cache_dir / "kma" / f"asos_{station}_{ymd}.json"
    if path.exists() and not refresh:
        return json.loads(path.read_text(encoding="utf-8"))["items"]
    key = os.environ.get("DATA_GO_KR_KEY", "")
    if not key:
        raise KmaError("환경변수 DATA_GO_KR_KEY가 없음")
    q = urllib.parse.urlencode({
        "pageNo": "1", "numOfRows": "24", "dataType": "JSON", "dataCd": "ASOS", "dateCd": "HR",
        "startDt": ymd, "startHh": "00", "endDt": ymd, "endHh": "23", "stnIds": station,
    })
    req = urllib.request.Request(f"{BASE}?serviceKey={key}&{q}", headers={"User-Agent": USER_AGENT})
    try:
        with urllib.request.urlopen(req, timeout=60) as r:
            body = r.read().decode("utf-8", "replace")
    except urllib.error.HTTPError as e:
        raise KmaError(f"HTTP {e.code} (활용 승인 여부 확인)") from None
    try:
        d = json.loads(body)
    except ValueError:
        raise KmaError("JSON이 아닌 응답") from None
    header = d.get("response", {}).get("header", {})
    if header.get("resultCode") != "00":
        raise KmaError(f"결과 코드 {header.get('resultCode')} {header.get('resultMsg')}")
    items = d["response"].get("body", {}).get("items", {}).get("item", [])
    keep = [{k: it.get(k) for k in ("tm", "stnId", "stnNm", "ta", "rn", "hm", "ws")} for it in items]
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps({"source": SOURCE, "items": keep}, ensure_ascii=False), encoding="utf-8")
    return keep


def to_event(items: list[dict], name: str, year: int, season: int) -> dict:
    """시간 자료 → schema/climate의 사건(events[]) 하나. 강수 없음은 0, 기온은 그날 최고."""
    rain = [0.0] * 24
    tmax = None
    for it in items:
        hh = int(str(it["tm"])[-5:-3])
        try:
            rain[hh] = float(it.get("rn") or 0.0)
        except ValueError:
            rain[hh] = 0.0
        try:
            t = float(it.get("ta"))
            tmax = t if tmax is None else max(tmax, t)
        except (TypeError, ValueError):
            pass
    stn = items[0].get("stnNm") if items else "?"
    return {"name": name, "year": year, "season": season, "rain_mm_per_h": rain, "tmax_c": tmax,
            "source": f"{SOURCE} ({stn} 관측소)"}
