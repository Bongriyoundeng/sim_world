"""헤드리스 실험: 도로 하나 막았을 때 통근 시간 변화 (이슈 #11).

평택역 맵 출퇴근 하루(tests/scenarios/scenario.py와 같은 설정)에서, 기준 하루에 실제로 많이 쓰인
도로 상위 N개를 하나씩 막고 평균 통근 시간 변화를 비교한다. 결과는 Markdown 표로 출력한다.

실행 (저장소 루트, core/crates/py/README.md대로 바인딩 설치 후):

    core/crates/py/.venv/Scripts/python ai/experiments/road_closure.py --top 5
    core/crates/py/.venv/Scripts/python ai/experiments/road_closure.py --top 5 --out ai/experiments/results/road_closure.md

같은 시드면 같은 표가 나온다 (코어가 결정론적).
"""

from __future__ import annotations

import argparse
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "tests" / "scenarios"))
from scenario import SEED, commute_day  # noqa: E402


def traffic(sim) -> dict:
    return sim.snapshot(vehicles=False)["traffic"]


def run(top: int, seed: int) -> str:
    t0 = time.perf_counter()
    base = commute_day(seed=seed)
    b = traffic(base)
    rows = []
    for rank, (edge, users) in enumerate(base.edge_usage(top=top), start=1):
        info = base.edge_info(edge)
        closed = commute_day(closed_edges=(edge,), seed=seed)
        c = traffic(closed)
        res = closed.snapshot(vehicles=False)["outcomes"][-1]["result"]
        rows.append(
            {
                "rank": rank,
                "edge": edge,
                "road": f"{info['name'] or '(이름 없음)'} ({info['highway']})",
                "length_m": info["length_m"],
                "lanes": f"{info['lanes_forward']}+{info['lanes_backward']}",
                "users": users,
                "rerouted": res.get("rerouted", 0),
                "mean": c["mean_travel_s"],
                "delta": c["mean_travel_s"] - b["mean_travel_s"],
                "max": c["max_travel_s"],
                "arrived": f"{c['arrived']}/{c['vehicles']}",
            }
        )
    elapsed = time.perf_counter() - t0

    lines = [
        f"기준: 통행 {b['vehicles']}개, 평균 통근 {b['mean_travel_s']:.1f}초, 최장 {b['max_travel_s']:.1f}초 "
        f"(시드 {seed}, 실행 {elapsed:.0f}초)",
        "",
        "| 순위 | 엣지 | 도로 | 길이(m) | 차선(정+역) | 기준 통과 차량 | 경로 바꾼 통행 | 평균 통근(초) | 변화(초) | 변화(%) | 최장(초) | 도착 |",
        "| ---: | ---: | --- | ---: | :---: | ---: | ---: | ---: | ---: | ---: | ---: | :---: |",
    ]
    for r in rows:
        lines.append(
            f"| {r['rank']} | {r['edge']} | {r['road']} | {r['length_m']:.0f} | {r['lanes']} | {r['users']:,} | "
            f"{r['rerouted']:,} | {r['mean']:.1f} | {r['delta']:+.1f} | {r['delta'] / b['mean_travel_s']:+.1%} | "
            f"{r['max']:.1f} | {r['arrived']} |"
        )
    return "\n".join(lines) + "\n"


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--top", type=int, default=5, help="막아 볼 상위 혼잡 도로 수")
    p.add_argument("--seed", type=int, default=SEED)
    p.add_argument("--out", type=Path, help="Markdown 표를 저장할 파일")
    args = p.parse_args()
    table = run(args.top, args.seed)
    print(table)
    if args.out:
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(table, encoding="utf-8")
    return 0


if __name__ == "__main__":
    sys.exit(main())
