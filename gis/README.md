# gis/ — 지도 데이터 변환 (Python)

동네 범위를 넣으면 OSM(도로·건물)과 Copernicus DEM(지형)을 받아 [맵 패키지](../schema/map-package/README.md)를 만듭니다.

```sh
cd gis && uv run pytest
```

0단계 4번 작업에서 내려받기(fetch), 도로(roads), 건물(buildings), 지형(terrain) 모듈을 추가합니다.
구글 지도 등 재배포가 허용되지 않는 데이터는 쓰지 않습니다 ([클린룸 규칙](../docs/legal/clean-room.md)).
