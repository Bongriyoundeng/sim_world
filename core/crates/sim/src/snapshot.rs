//! 스냅샷: 코어 → 클라이언트·AI로 보내는 현재 상태 (JSON). 형식: schema/snapshots/README.md

use serde_json::{json, Value};

use crate::traffic::VehicleState;
use crate::{Simulation, TICK_MS};

fn state_name(s: VehicleState) -> &'static str {
    match s {
        VehicleState::Scheduled => "scheduled",
        VehicleState::WaitingToEnter => "waiting_to_enter",
        VehicleState::Crossing => "crossing",
        VehicleState::Queued => "queued",
        VehicleState::WaitingToAdvance => "waiting_to_advance",
        VehicleState::Arrived => "arrived",
        VehicleState::Cancelled => "cancelled",
    }
}

impl Simulation {
    /// 지금 상태의 스냅샷. `include_vehicles`가 false면 차량 목록을 빼고 통계만 담는다.
    pub fn snapshot(&self, include_vehicles: bool) -> Value {
        let stats = self.traffic.stats();
        let g = &self.world.road_graph;
        let gs = g.stats();
        let mut snap = json!({
            "format_version": 0,
            "map": self.world.manifest.name,
            "tick": self.tick(),
            "time_s": (self.tick() * TICK_MS) as f64 / 1000.0,
            "day": self.day(),
            "seasons_completed": self.calendar.seasons.len(),
            "traffic": {
                "vehicles": stats.vehicles,
                "arrived": stats.arrived,
                "mean_travel_s": stats.mean_travel_s,
                "max_travel_s": stats.max_travel_s,
                "jam_relief": stats.jam_relief,
                "cancelled": stats.cancelled,
            },
            "road_graph": {
                "nodes": gs.nodes,
                "edges": gs.edges,
                "lanes": gs.lanes,
                "lane_km": gs.lane_km,
            },
            "zones": self.world.zones.zoned().count(),
            "outcomes": serde_json::to_value(&self.outcomes).expect("직렬화 가능"),
        });
        if include_vehicles {
            let vehicles: Vec<Value> = self
                .traffic
                .vehicles
                .iter()
                .map(|v| {
                    json!({
                        "id": v.id,
                        "state": state_name(v.state),
                        "origin": v.origin,
                        "dest": v.dest,
                        "lane": v.lane,
                        "edge": v.lane.map(|l| g.lanes[l as usize].edge),
                        "depart_s": v.depart_ms as f64 / 1000.0,
                        "arrive_s": v.arrive_ms.map(|a| a as f64 / 1000.0),
                    })
                })
                .collect();
            snap["vehicles"] = Value::Array(vehicles);
        }
        snap
    }
}
