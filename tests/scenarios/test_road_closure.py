"""시나리오: 가장 붐비는 도로를 막으면 평균 통근 시간이 늘어야 한다 (이슈 #12).

평택역 맵, 출퇴근 수요(시드 고정), 차단 전후를 비교한다.
"""

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).parent))
from scenario import commute_day, mean_commute_s  # noqa: E402


@pytest.fixture(scope="module")
def baseline():
    return commute_day()


def all_arrived(sim) -> bool:
    t = sim.snapshot(vehicles=False)["traffic"]
    return t["vehicles"] > 0 and t["arrived"] == t["vehicles"]


def test_baseline_completes(baseline):
    assert all_arrived(baseline)
    assert mean_commute_s(baseline) > 0


def test_closing_busiest_road_increases_mean_commute(baseline):
    busiest, users = baseline.edge_usage(top=1)[0]
    closed = commute_day(closed_edges=(busiest,))
    out = closed.snapshot(vehicles=False)["outcomes"]
    assert out[-1]["result"]["status"] == "applied"
    assert out[-1]["result"]["rerouted"] > 0, "차단한 도로를 지나던 통행은 경로를 바꿔야 함"
    assert all_arrived(closed), "막아도 다른 길로 모두 도착해야 함"

    before, after = mean_commute_s(baseline), mean_commute_s(closed)
    info = baseline.edge_info(busiest)
    print(f"차단 엣지 {busiest} ({info['highway']}, {info['name']}, 통과 {users}대): "
          f"평균 통근 {before:.1f}초 → {after:.1f}초 ({after - before:+.1f}초)")
    assert after > before


def test_scenario_is_deterministic(baseline):
    again = commute_day()
    assert again.snapshot() == baseline.snapshot()
