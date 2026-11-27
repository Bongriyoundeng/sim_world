# Godot 바인딩 (GDExtension, godot-rust/gdext)

`godot` crate 0.5.6, `api-4-3`(Godot 4.3 이상에서 같은 라이브러리). 클래스 `DongneWorld`(RefCounted)를 등록합니다.

| 메서드 | 뜻 |
| --- | --- |
| `load_map(path) -> bool`, `last_error()`, `is_loaded()` | 맵 패키지 폴더(절대 경로) 읽기 |
| `map_name()`, `extent() -> Vector2` | 이름, 범위(m) |
| `terrain_file()`, `terrain_range() -> Vector2(최저, 최고)`, `terrain_resolution()` | 높이맵 정보 |
| `road_count()`, `road_points(e)`, `road_highway(e)`, `road_lanes(e)`, `road_removed(e)` | 도로 엣지 |
| `building_count()`, `building_footprint(i)`, `building_height(i)` | 건물 바닥·표시 높이 |
| `step(n_ticks)`, `tick()`, `time_s()` | 시뮬레이션 진행 (`step(0)`은 쌓인 명령만 적용) |
| `vehicle_positions()` | 도로 위 차량 `[id, x, y, 진행 방향, …]` (차선 중심, 우측 통행) |
| `submit(json) -> id`, `outcomes_json()` | 명령 넣기(형식 오류면 -1), 처리 결과 |
| `zone_cells()`, `cell_at(p)` | 구역 셀 `[행, 열, 구역, …]`, 좌표의 셀 |
| `nearest_edge(p, max_m)` | 가까운 도로 엣지 (없으면 -1) |
| `add_random_trips(n, seed, window_s)`, `add_commute(seed)`, `traffic_json()` | 통행 만들기, 교통 통계 |

빌드와 복사: `python tools/build_godot_addon.py` → `client/addons/dongne_core/bin/` (커밋 안 함). 빌드가 무거워 workspace `default-members`에서 빠져 있고, CI는 `cargo clippy -p dongne-godot`으로 컴파일을 확인합니다.
