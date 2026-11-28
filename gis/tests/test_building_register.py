"""건축물대장 통계 (네트워크 없음): 주용도 → 시뮬레이션 용도, 연면적·층수·세대 합계, 키 없을 때 오류."""

import pytest

from dongne_gis import building_register as br


def item(purpose, floors, area, hh=0):
    return {"mainPurpsCdNm": purpose, "grndFlrCnt": floors, "totArea": area, "hhldCnt": hh, "fmlyCnt": 0, "hoCnt": 0}


def test_summarize_groups_by_use():
    s = br.summarize([
        item("단독주택", 2, 100.0, 1),
        item("단독주택", 1, 60.0, 1),
        item("제1종근린생활시설", 3, 240.0),
        item("업무시설", 10, 5000.0),
        item("알수없는용도", 1, 10.0),
    ])
    assert s["records"] == 5
    u = s["by_use"]
    assert u["residential"] == {"buildings": 2, "tot_area_m2": 160.0, "households": 2.0, "mean_floors": 1.5}
    assert u["commercial"]["tot_area_m2"] == 240.0 and u["office"]["mean_floors"] == 10.0
    assert u["other"]["buildings"] == 1
    assert s["residential_share_of_res_and_com"] == round(160 / 400, 3)
    assert s["floor_histogram"] == {1: 2, 2: 1, 3: 1, 10: 1}


def test_missing_key_raises_without_network(tmp_path, monkeypatch):
    monkeypatch.delenv("DATA_GO_KR_KEY", raising=False)
    with pytest.raises(br.RegisterError):
        br.fetch_dong("41220", "11300", cache_dir=tmp_path)


def test_cache_is_used_and_holds_no_key(tmp_path, monkeypatch):
    import json

    d = tmp_path / "building-register"
    d.mkdir()
    (d / "title_41220_11300.json").write_text(json.dumps({"items": [item("단독주택", 1, 50.0)]}), encoding="utf-8")
    monkeypatch.delenv("DATA_GO_KR_KEY", raising=False)
    got = br.fetch_dong("41220", "11300", cache_dir=tmp_path)
    assert got[0]["mainPurpsCdNm"] == "단독주택"
