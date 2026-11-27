# 맵 패키지 v0 (초안)

맵 패키지는 폴더 하나입니다.

```
<동네이름>/
├─ manifest.json      메타데이터 (아래)
├─ terrain.png        16비트 그레이스케일 높이맵
├─ landcover.png      8비트 RGB 토지 피복 (선택)
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
  "landcover": {
    "file": "landcover.png", "resolution_m": 30,
    "channels": { "r": "impervious", "g": "canopy", "b": "water" }, "scale": 255
  },
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

## landcover.png (선택)

- `landcover` 블록은 선택 항목입니다. 없으면 토지 피복이 없는 맵이고, 코어는 이 항목 없이도 manifest를 읽습니다.
- 격자는 높이맵과 같습니다 (origin 중심, `resolution_m` 셀, 행 0 = 북쪽, 열 0 = 서쪽).
- 8비트 RGB이고, 채널 값 / `scale`(255)이 셀 면적 대비 비율 0~1입니다.

| 채널 | 이름 | 뜻 |
| --- | --- | --- |
| R | `impervious` | 불투수: 건물 지붕 + 도로(선을 폭만큼 버퍼) + 포장 면적(주차장 등) |
| G | `canopy` | 녹지·나무 수관: 숲, 나무(점·줄), 잔디, 공원, 정원 |
| B | `water` | 수면: 물 폴리곤과 하천 선 버퍼 |

- 세 값은 서로 겹칠 수 있습니다 (예: 도로 위 가로수). 합이 1을 넘을 수 있습니다.
- 추정 방법과 폭 기본값: `gis/src/dongne_gis/landcover.py`, 근거 [research-log.md](../../docs/design/research-log.md#road-width)
