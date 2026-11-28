"""기상청 ASOS 도구 (네트워크 없음): 시간 자료 → 사건 변환, 키 없을 때 오류, 캐시."""

import json

import pytest

from dongne_gis import kma


def test_to_event_fills_24_hours():
    items = [
        {"tm": "2024-07-18 09:00", "stnNm": "수원", "rn": "12.5", "ta": "24.1"},
        {"tm": "2024-07-18 10:00", "stnNm": "수원", "rn": "", "ta": "25.0"},
        {"tm": "2024-07-18 23:00", "stnNm": "수원", "rn": "3", "ta": None},
    ]
    e = kma.to_event(items, "검증 강우", 2024, 1)
    assert len(e["rain_mm_per_h"]) == 24
    assert e["rain_mm_per_h"][9] == 12.5 and e["rain_mm_per_h"][10] == 0.0 and e["rain_mm_per_h"][23] == 3.0
    assert e["tmax_c"] == 25.0 and "수원" in e["source"] and "기상청" in e["source"]


def test_missing_key_and_cache(tmp_path, monkeypatch):
    monkeypatch.delenv("DATA_GO_KR_KEY", raising=False)
    with pytest.raises(kma.KmaError):
        kma.fetch_day("119", "2024-07-18", cache_dir=tmp_path)
    d = tmp_path / "kma"
    d.mkdir()
    (d / "asos_119_20240718.json").write_text(json.dumps({"items": [{"tm": "2024-07-18 00:00"}]}), encoding="utf-8")
    assert kma.fetch_day("119", "2024-07-18", cache_dir=tmp_path) == [{"tm": "2024-07-18 00:00"}]
