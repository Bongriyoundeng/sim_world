# 데이터 출처 표기

이 프로젝트와 프로젝트가 만드는 맵 패키지는 아래 데이터를 사용합니다. 게임 화면의 크레딧과 각 맵 패키지의 `manifest.json`에도 같은 표기를 넣습니다.

## OpenStreetMap (도로·건물)

© OpenStreetMap contributors — https://www.openstreetmap.org/copyright

OpenStreetMap 데이터는 Open Database License(ODbL) 1.0으로 제공됩니다.
게임 화면은 산출물(Produced Work)이지만, `maps/`에 배포하는 변환 데이터는 파생 데이터베이스이므로 ODbL 1.0으로 배포합니다.

## Copernicus DEM GLO-30 (지형)

© DLR e.V. 2010-2014 and © Airbus Defence and Space GmbH 2014-2018 provided under COPERNICUS by the European Union and ESA; all rights reserved

> TODO: 공개 배포 전 Copernicus DEM 라이선스 원문의 표기 문구와 대조해 확인할 것.

## 공공데이터 (2단계, [ADR 0008](docs/adr/0008-phase2-data-sources.md))

쓰는 자료만 맵 패키지 `attribution`과 게임 크레딧에 넣습니다.

- 인구: 행정안전부, 「지역별(행정동) 성별 연령별 주민등록 인구수」, 공공데이터포털 (https://www.data.go.kr/data/15097972/fileData.do)
- 토지가치: 국토교통부, 「개별공시지가정보」, 공공데이터포털 (https://www.data.go.kr/data/15052266/fileData.do)
- 건물(쓸 때만): 국토교통부, 「건축HUB 건축물대장정보 서비스」, 공공데이터포털 (https://www.data.go.kr/data/15134735/openapi.do)
- 기상: 기상청 기상자료개방포털, 종관기상관측(ASOS) 자료 (https://data.kma.go.kr)

