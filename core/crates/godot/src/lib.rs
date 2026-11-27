//! Godot 바인딩. 클라이언트(GDScript)는 `DongneWorld`로 맵 패키지를 읽고 시뮬레이션을 진행한다.
//!
//! 좌표는 맵 패키지와 같은 로컬 미터(x 동쪽, y 북쪽). Godot 3D로 옮길 때는 (x, 높이, -y)로 쓴다.

use godot::prelude::*;

use dongne_sim_core::Simulation;

struct DongneExtension;

#[gdextension]
unsafe impl ExtensionLibrary for DongneExtension {}

/// 맵 패키지 하나와 그 시뮬레이션.
#[derive(GodotClass)]
#[class(base = RefCounted, init)]
pub struct DongneWorld {
    sim: Option<Simulation>,
    error: String,
    base: Base<RefCounted>,
}

fn v2(p: (f64, f64)) -> Vector2 {
    Vector2::new(p.0 as f32, p.1 as f32)
}

impl DongneWorld {
    fn sim(&self) -> Option<&Simulation> {
        self.sim.as_ref()
    }
}

#[godot_api]
impl DongneWorld {
    /// 맵 패키지 폴더(절대 경로)를 읽는다. 실패하면 false, 이유는 `last_error()`.
    #[func]
    fn load_map(&mut self, path: GString) -> bool {
        let dir = std::path::PathBuf::from(path.to_string());
        match dongne_world::load_map(&dir) {
            Ok(world) => {
                self.sim = Some(Simulation::new(world));
                self.error.clear();
                true
            }
            Err(e) => {
                self.sim = None;
                self.error = format!("{}: {e}", dir.display());
                false
            }
        }
    }

    #[func]
    fn last_error(&self) -> GString {
        GString::from(self.error.as_str())
    }

    #[func]
    fn is_loaded(&self) -> bool {
        self.sim.is_some()
    }

    #[func]
    fn map_name(&self) -> GString {
        self.sim()
            .map(|s| GString::from(s.world.manifest.name.as_str()))
            .unwrap_or_default()
    }

    /// 맵 범위 (m): (가로, 세로). origin이 가운데.
    #[func]
    fn extent(&self) -> Vector2 {
        self.sim()
            .map(|s| {
                v2((
                    s.world.manifest.extent_m.width,
                    s.world.manifest.extent_m.height,
                ))
            })
            .unwrap_or_default()
    }

    /// 높이맵 파일 이름 (맵 폴더 기준).
    #[func]
    fn terrain_file(&self) -> GString {
        self.sim()
            .map(|s| GString::from(s.world.manifest.terrain.file.as_str()))
            .unwrap_or_default()
    }

    /// 높이맵 고도 범위 (m): (최저, 최고). 픽셀 0 → 최저, 최댓값 → 최고.
    #[func]
    fn terrain_range(&self) -> Vector2 {
        self.sim()
            .map(|s| {
                let t = &s.world.manifest.terrain;
                v2((t.min_elevation_m, t.max_elevation_m))
            })
            .unwrap_or_default()
    }

    #[func]
    fn terrain_resolution(&self) -> f64 {
        self.sim()
            .map(|s| s.world.manifest.terrain.resolution_m)
            .unwrap_or(0.0)
    }

    /// 도로 엣지 수 (없앤 도로 포함, `road_removed`로 확인).
    #[func]
    fn road_count(&self) -> i64 {
        self.sim()
            .map(|s| s.world.road_graph.edges.len() as i64)
            .unwrap_or(0)
    }

    /// 도로 엣지의 선 (로컬 미터).
    #[func]
    fn road_points(&self, edge: i64) -> PackedVector2Array {
        let mut out = PackedVector2Array::new();
        if let Some(e) = self
            .sim()
            .and_then(|s| s.world.road_graph.edges.get(edge as usize))
        {
            for &p in &e.points {
                out.push(v2(p));
            }
        }
        out
    }

    /// 도로 유형 (OSM highway).
    #[func]
    fn road_highway(&self, edge: i64) -> GString {
        self.sim()
            .and_then(|s| s.world.road_graph.edges.get(edge as usize))
            .map(|e| GString::from(e.highway.as_str()))
            .unwrap_or_default()
    }

    /// 양방향 합계 차선 수.
    #[func]
    fn road_lanes(&self, edge: i64) -> i64 {
        self.sim()
            .and_then(|s| s.world.road_graph.edges.get(edge as usize))
            .map(|e| (e.lanes_forward + e.lanes_backward) as i64)
            .unwrap_or(0)
    }

    #[func]
    fn road_removed(&self, edge: i64) -> bool {
        self.sim()
            .and_then(|s| s.world.road_graph.edges.get(edge as usize))
            .is_none_or(|e| e.removed)
    }

    #[func]
    fn building_count(&self) -> i64 {
        self.sim()
            .map(|s| s.world.buildings.len() as i64)
            .unwrap_or(0)
    }

    /// 건물 바닥 다각형 (로컬 미터, 닫는 점 없음).
    #[func]
    fn building_footprint(&self, i: i64) -> PackedVector2Array {
        let mut out = PackedVector2Array::new();
        if let Some(b) = self.sim().and_then(|s| s.world.buildings.get(i as usize)) {
            for &p in &b.footprint {
                out.push(v2(p));
            }
        }
        out
    }

    /// 건물 표시 높이 (m). 태그가 없으면 유형별 기본값(가정).
    #[func]
    fn building_height(&self, i: i64) -> f64 {
        self.sim()
            .and_then(|s| s.world.buildings.get(i as usize))
            .map(|b| b.height_m)
            .unwrap_or(0.0)
    }

    /// n틱(1틱 = 100 ms) 진행.
    #[func]
    fn step(&mut self, n_ticks: i64) {
        if let Some(s) = self.sim.as_mut() {
            s.step(n_ticks.max(0) as u64);
        }
    }

    #[func]
    fn tick(&self) -> i64 {
        self.sim().map(|s| s.tick() as i64).unwrap_or(0)
    }
}
