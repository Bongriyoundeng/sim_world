"""밸런스 실험: 주거 셀당 통근자 수에 따라 통근 시간과 혼잡이 어떻게 변하나 (이슈 #13).

tests/scenarios/scenario.py와 같은 구역(주거 169셀·업무 100셀·상업 100셀)에서 셀당 통근자 수만 바꿔
출퇴근 하루를 돌리고, 평균·최장 통근 시간, 도착률, 08시 도로 위 차량 수를 표로 낸다.

    core/crates/py/.venv/Scripts/python ai/experiments/balance_sweep.py
    core/crates/py/.venv/Scripts/python ai/experiments/balance_sweep.py --residents 1 5 10 20 40 --out ai/experiments/results/balance_sweep.md
"""

from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

import dongne_core

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests" / "scenarios"))
from scenario import DAY_TICKS, MAP, SEED, ZONES  # noqa: E402

HOUR = 3600.0


def day(residents: int, seed: int) -> tuple[dict, dict, list]:
    sim = dongne_core.Simulation.load(str(MAP))
    for area, zone in ZONES:
        sim.submit({"type": "set_zone", "area": area, "zone": zone})
    sim.step()
    report = sim.add_commute(seed=seed, residents_per_cell=residents)
    while sim.tick < DAY_TICKS:
        sim.step(36_000)
    snap = sim.snapshot()
    return report, snap["traffic"], snap["vehicles"]


def on_road(vehicles: list, h0: float, h1: float) -> int:
    """[h0, h1) 시간대에 한 번이라도 도로 위에 있던 차량 수."""
    return sum(
        1
        for v in vehicles
        if v["depart_s"] < h1 * HOUR and (v["arrive_s"] if v["arrive_s"] is not None else 1e18) >= h0 * HOUR
    )


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--residents", type=int, nargs="+", default=[1, 5, 10, 20, 40])
    p.add_argument("--seed", type=int, default=SEED)
    p.add_argument("--out", type=Path)
    args = p.parse_args()

    rows = []
    free_flow = None
    for r in args.residents:
        t0 = time.perf_counter()
        rep, tr, vs = day(r, args.seed)
        if free_flow is None:
            free_flow = tr["mean_travel_s"]
        rows.append(
            f"| {r} | {rep['commuters']:,} | {rep['trips']:,} | {tr['arrived'] / max(tr['vehicles'], 1):.1%} | "
            f"{tr['mean_travel_s']:.1f} | {tr['mean_travel_s'] / free_flow:.2f} | {tr['max_travel_s']:.1f} | "
            f"{on_road(vs, 8, 9):,} | {time.perf_counter() - t0:.0f} |"
        )
        print(rows[-1], flush=True)

    table = "\n".join(
        [
            f"시드 {args.seed}, 일자리 업무 3,000 + 상업 1,000 = 4,000개 (통근자는 일자리 수를 넘지 않음). "
            f"'혼잡 배율' = 평균 통근 ÷ 첫 행 평균 통근.",
            "",
            "| 주거 셀당 통근자 | 통근자 | 통행 | 도착률 | 평균 통근(초) | 혼잡 배율 | 최장(초) | 08시 도로 위 차량 | 실행(초) |",
            "| ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |",
            *rows,
        ]
    )
    print("\n" + table)
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(table + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
