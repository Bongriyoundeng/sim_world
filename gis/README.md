# gis/ — 지도 데이터 변환 (Python)

동네 범위를 넣으면 OSM(도로·건물)과 Copernicus DEM(지형)을 받아 [맵 패키지](../schema/map-package/README.md)를 만듭니다.

```sh
cd gis && uv run pytest
```

## 맵 패키지 만들기 (도로·건물)

```sh
cd gis
uv run python -m dongne_gis build --name pyeongtaek-station --lat 36.993 --lon 127.087 --size 1500 --out ../maps/pyeongtaek-station
uv run python -m dongne_gis check --lat 36.993 --lon 127.087 --size 1500   # 평택역·통복천 포함 확인
```

| 모듈 | 하는 일 |
| --- | --- |
| `fetch.py` | Overpass API에서 bbox 안 `highway` way와 `building` way/relation을 한 번에 받음. 원본 응답은 저장소의 `data-raw/`에 캐시 (커밋 안 함, `--refresh`로 다시 받기) |
| `project.py` | 위경도 → origin 중심 횡메르카토르 미터 좌표 (x 동쪽, y 북쪽) |
| `roads.py` | `roads.geojson`: LineString, `osm_id`·`highway`·`name`·`lanes`·`oneway`. 구역 경계에서 자르고, `oneway=-1`은 진행 방향으로 뒤집음 |
| `buildings.py` | `buildings.geojson`: Polygon, `osm_id`·`osm_type`·`building`·`building:levels`·`height`. 무게중심이 구역 안인 건물만, 멀티폴리곤은 바깥 고리마다 하나 |
| `package.py` | `manifest.json`, 두 GeoJSON, 출처·생성 정보를 담은 `README.md` 생성 |

지형(`terrain.png`)은 이슈 #2에서 추가합니다. 그때까지 manifest의 고도 범위는 0입니다.
구글 지도 등 재배포가 허용되지 않는 데이터는 쓰지 않습니다 ([클린룸 규칙](../docs/legal/clean-room.md)).
