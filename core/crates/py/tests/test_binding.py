"""Python에서 코어를 불러 step·snapshot·명령이 동작하는지 확인한다."""

import json
from pathlib import Path

import pytest

import dongne_core

MAP = Path(__file__).resolve().parents[4] / "maps" / "pyeongtaek-station"


def run(seed: int) -> dongne_core.Simulation:
    sim = dongne_core.Simulation.load(str(MAP))
    assert sim.add_random_trips(100, seed=seed, depart_window_s=300) == 100
    while sim.snapshot(vehicles=False)["traffic"]["arrived"] < 100 and sim.time_s < 3 * 3600:
        sim.step(600)
    return sim


def test_step_and_snapshot():
    sim = run(42)
    snap = sim.snapshot()
    assert snap["map"] == "pyeongtaek-station"
    assert snap["tick"] == sim.tick and snap["time_s"] == sim.time_s
    assert snap["traffic"]["arrived"] == 100
    assert snap["traffic"]["mean_travel_s"] > 0
    assert len(snap["vehicles"]) == 100
    assert all(v["state"] == "arrived" and v["arrive_s"] >= v["depart_s"] for v in snap["vehicles"])
    assert snap["road_graph"]["nodes"] == 908
    json.dumps(snap)  # 그대로 JSON으로 보낼 수 있다


def test_deterministic_from_python():
    a = run(42).snapshot()
    b = run(42).snapshot()
    assert a == b
    c = run(43).snapshot()
    assert [v["origin"] for v in a["vehicles"]] != [v["origin"] for v in c["vehicles"]]


def test_commands_dict_and_json():
    sim = dongne_core.Simulation.load(str(MAP))
    i = sim.submit({"type": "set_zone", "area": {"cells": [[0, 0], [0, 1]]}, "zone": "residential"})
    j = sim.submit(json.dumps({"type": "remove_road", "edge": 999_999}))
    assert sim.snapshot()["outcomes"] == []  # 다음 step 전에는 적용 안 됨
    sim.step()
    out = sim.snapshot()["outcomes"]
    assert out[0] == {"id": i, "tick": 0, "result": {"status": "applied", "cells": 2}}
    assert out[1]["id"] == j and out[1]["result"]["status"] == "rejected"
    assert sim.snapshot()["zones"] == 2


def test_bad_command_and_missing_map():
    sim = dongne_core.Simulation.load(str(MAP))
    with pytest.raises(ValueError):
        sim.submit({"type": "fly"})
    with pytest.raises(OSError):
        dongne_core.Simulation.load("no/such/map")


def test_commute_from_python():
    sim = dongne_core.Simulation.load(str(MAP))
    rect = lambda x0, y0, x1, y1: {"polygon": [{"x": x0, "y": y0}, {"x": x1, "y": y0}, {"x": x1, "y": y1}, {"x": x0, "y": y1}]}
    sim.submit({"type": "set_zone", "area": rect(-700, -700, -400, -400), "zone": "residential"})
    sim.submit({"type": "set_zone", "area": rect(-250, -400, 50, -100), "zone": "office"})
    sim.step()
    r = sim.add_commute(seed=1, residents_per_cell=2)
    assert r["commuters"] > 0 and r["trips"] == 2 * (r["commuters"] - r["same_node"])
    assert "Simulation(map=" in repr(sim)
