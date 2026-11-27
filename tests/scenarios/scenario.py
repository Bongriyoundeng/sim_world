"""시나리오 공통 도구: 평택역 맵에 구역을 칠하고 출퇴근 하루를 돌린다 (dongne_core 사용).

시나리오 테스트(tests/scenarios)와 헤드리스 실험(ai/experiments)이 같은 설정을 쓴다.
"""

from __future__ import annotations

from pathlib import Path

import dongne_core

ROOT = Path(__file__).resolve().parents[2]
MAP = ROOT / "maps" / "pyeongtaek-station"
DAY_TICKS = 24 * 3600 * 1000 // dongne_core.TICK_MS
SEED = 2026


def rect(x0: float, y0: float, x1: float, y1: float) -> dict:
    return {"polygon": [{"x": x0, "y": y0}, {"x": x1, "y": y0}, {"x": x1, "y": y1}, {"x": x0, "y": y1}]}


# 남서쪽 주거, 역 앞 업무, 동쪽 상업 (core/crates/sim/tests/commute.rs와 같은 구역)
ZONES = [
    (rect(-700, -700, -300, -300), "residential"),
    (rect(-250, -400, 50, -100), "office"),
    (rect(200, 0, 500, 300), "commercial"),
]


def commute_day(closed_edges: tuple[int, ...] = (), seed: int = SEED) -> dongne_core.Simulation:
    """구역 → 출퇴근 통행 → (도로 차단) → 하루 실행한 시뮬레이션."""
    sim = dongne_core.Simulation.load(str(MAP))
    for area, zone in ZONES:
        sim.submit({"type": "set_zone", "area": area, "zone": zone})
    sim.step()
    sim.add_commute(seed=seed)
    for e in closed_edges:
        sim.submit({"type": "remove_road", "edge": e})
    while sim.tick < DAY_TICKS:
        sim.step(36_000)  # 1시간
    return sim


def mean_commute_s(sim: dongne_core.Simulation) -> float:
    return sim.snapshot(vehicles=False)["traffic"]["mean_travel_s"]
