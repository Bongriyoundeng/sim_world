//! Python 바인딩. 화면 없이 코어를 돌리는 실험(ai/)과 테스트용.
//!
//! ```python
//! import dongne_core
//! sim = dongne_core.Simulation.load("maps/pyeongtaek-station")
//! sim.add_random_trips(100, seed=42)
//! sim.submit({"type": "remove_road", "edge": 5})
//! sim.step(600)
//! snap = sim.snapshot()
//! ```

use std::path::PathBuf;

use pyo3::exceptions::{PyIOError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::PyString;

use dongne_api::Command;
use dongne_sim_core::demand::DemandParams;
use dongne_sim_core::{Simulation as CoreSimulation, TICK_MS};

/// 코어 시뮬레이션. 같은 맵·명령·시드면 같은 결과가 나온다.
#[pyclass(name = "Simulation", module = "dongne_core", unsendable)]
struct Simulation {
    inner: CoreSimulation,
}

/// 파이썬 값(JSON 문자열 또는 dict 등)을 JSON 문자열로 바꾼다.
fn to_json(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<String> {
    if let Ok(s) = value.cast::<PyString>() {
        return Ok(s.to_str()?.to_string());
    }
    let json = py.import("json")?;
    json.call_method1("dumps", (value,))?.extract()
}

fn from_json(py: Python<'_>, value: &serde_json::Value) -> PyResult<Py<PyAny>> {
    let json = py.import("json")?;
    Ok(json.call_method1("loads", (value.to_string(),))?.unbind())
}

#[pymethods]
impl Simulation {
    /// 맵 패키지 폴더를 읽어 시뮬레이션을 만든다.
    #[staticmethod]
    fn load(path: PathBuf) -> PyResult<Self> {
        let world = dongne_world::load_map(&path)
            .map_err(|e| PyIOError::new_err(format!("{}: {e}", path.display())))?;
        Ok(Self {
            inner: CoreSimulation::new(world),
        })
    }

    /// n틱(1틱 = 100 ms) 진행한다. 쌓인 명령이 먼저 적용된다.
    #[pyo3(signature = (n_ticks = 1))]
    fn step(&mut self, n_ticks: u64) {
        self.inner.step(n_ticks);
    }

    /// 지금까지 진행한 틱 수.
    #[getter]
    fn tick(&self) -> u64 {
        self.inner.tick()
    }

    /// 시뮬레이션 시각 (초, 자정 = 0).
    #[getter]
    fn time_s(&self) -> f64 {
        (self.inner.tick() * TICK_MS) as f64 / 1000.0
    }

    /// 명령(dict 또는 JSON 문자열, schema/commands 형식)을 넣고 번호를 돌려준다.
    /// 다음 step에서 적용되고 결과는 snapshot()["outcomes"]에 남는다.
    fn submit(&mut self, py: Python<'_>, command: &Bound<'_, PyAny>) -> PyResult<u64> {
        let json = to_json(py, command)?;
        let cmd: Command = serde_json::from_str(&json)
            .map_err(|e| PyValueError::new_err(format!("명령 형식 오류: {e}")))?;
        Ok(self.inner.submit(cmd))
    }

    /// 가장 큰 강연결 요소 안에서 무작위 통행 n개를 예약하고, 예약한 수를 돌려준다.
    #[pyo3(signature = (n, seed = 0, depart_window_s = 300.0))]
    fn add_random_trips(&mut self, n: usize, seed: u64, depart_window_s: f64) -> usize {
        self.inner
            .add_random_trips(n, seed, (depart_window_s * 1000.0) as u64)
    }

    /// 출발·도착 노드 사이 통행 하나를 예약한다. 경로가 없으면 None.
    #[pyo3(signature = (origin, dest, depart_s = 0.0))]
    fn add_trip(&mut self, origin: u32, dest: u32, depart_s: f64) -> Option<u32> {
        self.inner
            .traffic
            .add_trip(origin, dest, (depart_s * 1000.0) as u64)
    }

    /// 지금 구역 맵에서 출퇴근 통행을 만든다. 결과 요약을 dict로 돌려준다.
    #[pyo3(signature = (seed = 0, residents_per_cell = None, jobs_per_office_cell = None, jobs_per_commercial_cell = None))]
    fn add_commute(
        &mut self,
        py: Python<'_>,
        seed: u64,
        residents_per_cell: Option<u32>,
        jobs_per_office_cell: Option<u32>,
        jobs_per_commercial_cell: Option<u32>,
    ) -> PyResult<Py<PyAny>> {
        let d = DemandParams::default();
        let p = DemandParams {
            residents_per_cell: residents_per_cell.unwrap_or(d.residents_per_cell),
            jobs_per_office_cell: jobs_per_office_cell.unwrap_or(d.jobs_per_office_cell),
            jobs_per_commercial_cell: jobs_per_commercial_cell
                .unwrap_or(d.jobs_per_commercial_cell),
            ..d
        };
        let r = self.inner.add_commute(&p, seed);
        from_json(
            py,
            &serde_json::json!({
                "residents": r.demand.residents,
                "jobs": r.demand.jobs,
                "commuters": r.demand.commuters,
                "cells_without_road": r.demand.cells_without_road,
                "trips": r.trips,
                "same_node": r.same_node,
                "unroutable": r.unroutable,
            }),
        )
    }

    /// 건물 용도·연면적으로 오늘(지금 날짜 자정 기준) 하루의 출퇴근·방문 차량 통행을 만든다 (#36).
    /// 요약 dict: 거주자·종사자 수, 통행 수, 용도별 합계.
    #[pyo3(signature = (seed = 0, car_share = None))]
    fn add_building_demand(
        &mut self,
        py: Python<'_>,
        seed: u64,
        car_share: Option<f64>,
    ) -> PyResult<Py<PyAny>> {
        let mut lp = dongne_world::land_use::LandUseParams::default();
        if let Some(c) = car_share {
            lp.car_share = c;
        }
        let day_start = self.inner.day() as u64 * dongne_sim_core::demand::DAY_MS;
        let r = self
            .inner
            .add_building_demand_at(&lp, &DemandParams::default(), seed, day_start);
        let by_use: serde_json::Map<String, serde_json::Value> = r
            .by_use
            .iter()
            .map(|(u, t)| {
                (
                    format!("{u:?}").to_lowercase(),
                    serde_json::json!({
                        "buildings": t.buildings, "floor_area_m2": t.floor_area_m2,
                        "residents": t.residents, "workers": t.workers,
                        "trips_from": t.trips_from, "trips_to": t.trips_to,
                    }),
                )
            })
            .collect();
        from_json(
            py,
            &serde_json::json!({
                "residents": r.residents, "workers": r.workers,
                "car_commuters": r.car_commuters, "visit_trips": r.visit_trips,
                "trips": r.trips, "same_node": r.same_node, "unroutable": r.unroutable,
                "buildings_used": r.buildings_used, "buildings_without_road": r.buildings_without_road,
                "by_use": by_use,
            }),
        )
    }

    /// 다음 자정까지 진행하고 그날 보고서(dict)를 돌려준다.
    fn run_day(&mut self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let r = self.inner.run_day();
        from_json(py, &r.to_json())
    }

    /// 한 계절(대표 평일 하루를 차량 단위로 돌리고 90일로 집계)을 진행하고 보고서(dict)를 돌려준다.
    /// ADR 0007. 쌓인 명령은 계절 시작 때 적용된다.
    #[pyo3(signature = (seed = 0, residents_per_cell = None))]
    fn run_season(
        &mut self,
        py: Python<'_>,
        seed: u64,
        residents_per_cell: Option<u32>,
    ) -> PyResult<Py<PyAny>> {
        let d = DemandParams::default();
        let p = DemandParams {
            residents_per_cell: residents_per_cell.unwrap_or(d.residents_per_cell),
            ..d
        };
        let r = self.inner.run_season(&p, seed);
        from_json(py, &r.to_json())
    }

    /// 지금 날짜 (0부터).
    #[getter]
    fn day(&self) -> u32 {
        self.inner.day()
    }

    /// 결정론 상태 해시 (16자리 16진수). 같은 맵·명령·시드면 OS와 무관하게 같다.
    fn state_hash(&self) -> String {
        self.inner.state_hash_hex()
    }

    /// 엣지별 실제 통과 차량 수 `[(edge, vehicles), ...]`, 많은 순. `top`을 주면 앞에서 그만큼만.
    #[pyo3(signature = (top = None))]
    fn edge_usage(&self, top: Option<usize>) -> Vec<(u32, u32)> {
        let mut u = self.inner.edge_usage();
        if let Some(n) = top {
            u.truncate(n);
        }
        u
    }

    /// 엣지 정보 dict (도로 유형, 이름, 길이, 방향별 차선 수, 없앤 도로인지).
    fn edge_info(&self, py: Python<'_>, edge: u32) -> PyResult<Py<PyAny>> {
        let e = self
            .inner
            .world
            .road_graph
            .edges
            .get(edge as usize)
            .ok_or_else(|| PyValueError::new_err(format!("없는 엣지: {edge}")))?;
        from_json(
            py,
            &serde_json::json!({
                "edge": e.id, "osm_id": e.osm_id, "highway": e.highway, "name": e.name,
                "length_m": e.length_m, "lanes_forward": e.lanes_forward,
                "lanes_backward": e.lanes_backward, "removed": e.removed,
            }),
        )
    }

    /// 지금 상태를 dict로 (schema/snapshots 형식). vehicles=False면 차량 목록을 뺀다.
    #[pyo3(signature = (vehicles = true))]
    fn snapshot(&self, py: Python<'_>, vehicles: bool) -> PyResult<Py<PyAny>> {
        from_json(py, &self.inner.snapshot(vehicles))
    }

    fn __repr__(&self) -> String {
        format!(
            "Simulation(map={:?}, tick={})",
            self.inner.world.manifest.name,
            self.inner.tick()
        )
    }
}

#[pymodule]
fn dongne_core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<Simulation>()?;
    m.add("TICK_MS", TICK_MS)?;
    m.add("__version__", env!("CARGO_PKG_VERSION"))?;
    Ok(())
}
