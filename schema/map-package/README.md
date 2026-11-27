# 맵 패키지 v0 (초안)

맵 패키지는 폴더 하나입니다.

```
<동네이름>/
├─ manifest.json      메타데이터 (아래)
├─ terrain.png        16비트 그레이스케일 높이맵
├─ roads.geojson      도로 (LineString, 로컬 미터 좌표)
└─ buildings.geojson  건물 (Polygon, 로컬 미터 좌표)
```

## manifest.json

```json
{
  "format_version": 0,
  "name": "sample-dongne",
  "origin": { "lat": 37.5, "lon": 127.0 },
  "crs": "local-meters",
  "extent_m": { "width": 1500, "height": 1500 },
  "terrain": { "file": "terrain.png", "resolution_m": 30, "min_elevation_m": 0, "max_elevation_m": 120 },
  "attribution": [
    "© OpenStreetMap contributors (ODbL 1.0)",
    "© DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved"
  ]
}
```

- 좌표는 `origin`을 원점(0, 0)으로 하는 미터 단위, x는 동쪽, y는 북쪽입니다.
- 높이맵 픽셀 값 0~65535를 `min_elevation_m`~`max_elevation_m`에 선형으로 대응합니다.
- 높이맵 격자는 `origin`을 중심으로 `extent_m`을 덮고, 셀 크기는 `resolution_m`입니다 (1500m, 30m → 50×50). **행 0 = 북쪽 가장자리, 열 0 = 서쪽 가장자리**이고, 각 픽셀은 셀 중심의 고도입니다. 행 r, 열 c 셀 중심의 로컬 좌표는 x = −width/2 + (c + 0.5)·res, y = height/2 − (r + 0.5)·res 입니다.
- `attribution`이 비어 있으면 코어가 로드를 거부합니다.
