# client/ — Godot 4 클라이언트

화면과 UI를 맡습니다. 무거운 계산은 코어가 하고, 클라이언트는 결과를 그리고 입력을 명령으로 바꿉니다.

- Godot 4.3 이상 + GDScript (이 PC에서는 4.7.2로 확인)
- 코어는 GDExtension(`addons/dongne_core/`, 클래스 `DongneWorld`)으로 불러옵니다. 라이브러리 빌드물은 커밋하지 않습니다.

## 실행

```sh
python tools/build_godot_addon.py                 # core/crates/godot 빌드 → addons/dongne_core/bin/
godot --path client --import --headless           # 처음 한 번 (리소스 가져오기)
godot --path client                               # 맵 보기 창
godot --headless --path client -- --quit-after-load   # 화면 없이 씬 로드 확인 → "MAP_VIEW_OK …"
godot --path client -- --capture map.png          # 화면 캡처 저장 후 종료
godot --path client -- --map D:/other/map-package  # 다른 맵 패키지
godot --headless --path client -- --selftest     # 명령·차량 위치·보간 확인 → "SELFTEST_OK …"
godot --path client -- --demo frames/            # 출퇴근 장면을 돌리며 프레임 PNG 저장
```

## 플레이 (`scripts/sim_controller.gd`)

| 키·마우스 | 동작 |
| --- | --- |
| `1` | 보기 |
| `2` + 클릭 두 번 | 도로 그리기 (양방향 2차로 residential, 끝점은 5 m 안의 노드에 붙음) |
| `3` + 클릭 | 가장 가까운 도로(20 m 안) 삭제 |
| `4`~`7` + 끌기 | 구역 칠하기: 주거·상업·업무·공원 |
| `8` + 끌기 | 구역 지우기 |
| `C` | 지금 구역으로 출퇴근 통행 만들기 |
| `T` | 무작위 차량 200대 (2분 안에 출발) |
| `Space`, `+`/`-` | 멈춤·재생, 속도 (실시간 1초당 시뮬레이션 1~300초) |
| `W A S D`, 휠, 오른쪽 끌기 | 카메라 이동, 확대·축소, 회전 |

- 도구는 모두 코어 명령(`schema/commands`)으로 바뀌어 다음 틱 경계에 적용되고, 결과(적용·거부)가 화면 위쪽에 나옵니다. 멈춰 있어도 바로 적용됩니다.
- **차량 보간**: 코어를 0.5초(5틱)씩 진행하고, 그 사이 프레임은 앞뒤 위치를 선형 보간합니다(진행 방향은 각도 보간). 차량 위치는 코어의 `lane_positions`가 차선 안 진행률과 대기열 순번(앞차 뒤 7.5 m)으로 계산합니다. 차량은 잘 보이게 실제보다 조금 크게(6 × 3 m) 그립니다.
- 구역은 반투명 색 사각형으로 표시합니다 (주거 초록, 상업 파랑, 업무 주황, 공원 진초록).

## 맵 보기 (`scenes/map_view.tscn`, `scripts/map_view.gd`)

| 레이어 | 데이터 | 표시 |
| --- | --- | --- |
| 지형 | `terrain.png` + manifest 고도 범위 | 셀 중심을 꼭짓점으로 하는 격자 메시, 높이에 따라 색 |
| 도로 | 코어 도로 그래프 (`road_points`, 차선 수, 유형) | 차선 수 × 3 m 폭의 띠, 지형 위 0.6 m, 유형별 색 |
| 건물 | 코어가 읽은 `buildings.geojson` | 바닥 다각형을 표시 높이만큼 올린 상자 (지붕 삼각분할 + 벽) |

- 좌표: 맵 패키지 로컬 미터 (x 동쪽, y 북쪽) → Godot `(x, 높이, -y)`.
- 건물 높이는 OSM에 층수가 거의 없어 유형별 **가정** 높이를 씁니다 ([research-log.md](../docs/design/research-log.md#building-height)).
- Godot는 16비트 PNG를 8비트로 읽어서 지형 높이는 약 (최고−최저)/255 m 단위입니다 (평택역 맵 0.17 m). 표시에는 충분하고, 시뮬레이션은 코어가 원본을 씁니다.

![평택역 맵 보기](../docs/images/pyeongtaek-station-map-view.png)

캡처: 지도 데이터 © OpenStreetMap contributors (ODbL), 지형 © Copernicus DEM — [ATTRIBUTION.md](../ATTRIBUTION.md)
