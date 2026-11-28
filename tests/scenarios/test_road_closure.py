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
    """기준 하루에 많이 쓰인 도로 중, 막아도 모든 통행이 갈 수 있는(통행 포기 0) 가장 붐비는 도로를 막는다.

    도로를 막아 어떤 집이 완전히 끊기는 경우(통행 포기)는 '우회 → 통근 시간 증가'와 다른 질문이라 고르지 않는다.
    """
    chosen = None
    for edge, users in baseline.edge_usage(top=10):
        closed = commute_day(closed_edges=(edge,))
        t = closed.snapshot(vehicles=False)["traffic"]
        if t["cancelled"] == 0 and all_arrived(closed):
            chosen = (edge, users, closed)
            break
    assert chosen is not None, "상위 10개 도로 중 막아도 모두 도착하는 도로가 있어야 함"
    busiest, users, closed = chosen
    out = closed.snapshot(vehicles=False)["outcomes"]
    assert out[-1]["result"]["status"] == "applied"
    assert all(e != busiest for e, _ in closed.edge_usage()), "막은 도로는 아무도 지나지 않음"

    before, after = mean_commute_s(baseline), mean_commute_s(closed)
    info = baseline.edge_info(busiest)
    print(f"차단 엣지 {busiest} ({info['highway']}, {info['name']}, 통과 {users}대): "
          f"평균 통근 {before:.1f}초 → {after:.1f}초 ({after - before:+.1f}초)")
    assert after > before


def test_scenario_is_deterministic(baseline):
    again = commute_day()
    assert again.snapshot() == baseline.snapshot()
