# 근거 기록 (Research Log)

기획 결정마다 근거 자료를 여기 남깁니다. 각 항목은 직접 열어 확인한 페이지만 적습니다. 조사일: 2026-11-27.

## 기후 위험의 크기 (한국)

<a id="heat-korea"></a>
**폭염 — 온열질환 감시 결과**
- 2024-05-20 ~ 08-19 누적 온열질환자 2,890명, 추정 사망 26명. 2011년 감시 시작 이래 두 번째. 최다는 2018년 4,526명(사망 48명). 남성 77.2%, 실외 발생 78.5%. 출처: 질병관리청 응급실감시체계, [주간경향 2024-08-20](https://weekly.khan.co.kr/article/202408201745001)
- 쓰인 곳: 위험 우선순위(ADR 0003), 실외 노동자·고령자 가중치(climate.md)

<a id="flood-seoul-2022"></a>
**침수 — 2022년 8월 서울 집중호우**
- 동작구 신대방동 1시간 141.5mm(기상청 지역 관측, 서울 공식값은 종로구 송월동). 서울 일 강수 381.5mm로 1907년 관측 이래 최고. 관악구 반지하 주택에서 3명 사망. 이재민 391명, 주택·상가 침수 741건. 출처: [Korea Herald via ANN, 2022-08-10](https://asianews.network/heaviest-rainfall-in-115-years-wreaks-havoc-on-greater-seoul)
- 쓰인 곳: 위험 우선순위, 반지하 가구 피해 가중치

## 열섬 완화 효과

<a id="heat-trees"></a>
**Kumar et al. (2024)** "Urban heat mitigation by green and blue infrastructure: Drivers, effectiveness, and future needs." *The Innovation* 5(2):100588. [PMC10909648](https://pmc.ncbi.nlm.nih.gov/articles/PMC10909648/)
- 관측된 최대 기온 저감: 식물원 5.0 ± 3.5 °C, 습지 4.9 ± 3.2 °C, 녹색 벽 4.1 ± 4.2 °C, 가로수 3.8 ± 3.1 °C.
- 본문 인용: 가로수 주간 기온 최대 1.5 °C 저감(Coutts et al.).
- 쓰인 곳: 냉각 기본값. 최대값이 아닌 보수적 값(1.0~1.5 °C)을 기본으로 택함 — 게임이 효과를 과장하지 않게.

**Li et al. (2024)** "Cooling efficacy of trees across cities is determined by background climate, urban morphology, and tree trait." *Communications Earth & Environment* 5:754. [PDF](https://heathealth.info/wp-content/uploads/s43247-024-01908-4.pdf)
- 보행자 높이 최대 12 °C 저감. 상록·낙엽 혼합이 단일 수종보다 약 0.5 °C 더 냉각.
- 쓰인 곳: 혼합 수종 보너스(+0.5 °C), 효과가 배경 기후·도시 형태에 따라 달라진다는 점 → 동네마다 계수 보정 여지.

## 침수 유출

<a id="runoff"></a>
**Iowa DOT Design Manual 4A-5, Rational Method** [링크](https://iowadot.gov/design-manual/chapter-4-drainage/4a-5-using-rational-method-determine-peak-flow)
- Q = CIA. 유출계수(5년 빈도): 포장·건물 0.94, 상업 0.81, 연립 주거 0.66, 단독 주거 0.48, 잔디 0.22, 공원 0.21.
- 쓰인 곳: 침수 모델 1단계. 한국 기준 값으로 교체 가능한지는 조사 과제.

## 게임 디자인

<a id="serious-games"></a>
**Neset et al. (2020)** "Serious Gaming for Climate Adaptation—Assessing the Potential and Challenges of a Digital Serious Game for Urban Climate Adaptation." *Sustainability* 12(5):1789. [링크](https://www.mdpi.com/2071-1050/12/5/1789/htm)
- 게임이 적응 의사결정의 어려움을 성찰하게 하지만, 복잡도가 높으면 개별 조치의 결과를 파악하기 어렵다. 학습은 대부분 디브리핑에서 일어난다.
- 쓰인 곳: 루프의 "보좌관 브리핑" 단계, 기후 위험 2개로 제한, 지표 5개 상한.

## 교통 시뮬레이션 방식 (1단계)

<a id="traffic-model"></a>
**A/B Street — Discrete event traffic simulation** (Dustin Carlino, 2021) [링크](https://a-b-street.github.io/docs/tech/trafficsim/discrete_event/index.html)
- 0.1초마다 모든 차량을 갱신하는 방식은 느리고 부동소수점 예외가 많았고, 부드러운 가감속은 핵심 아이디어에 가치를 더하지 않아 이벤트 방식(우선순위 큐)으로 바꿨다. 차량은 차선별 대기열로 움직이고, 충돌은 교차로에서 처리.
- 대가: 가감속·안전거리가 없음, 차선 변경은 주로 교차로에서.

**FHWA Traffic Analysis Toolbox Vol. III, Appendix A** [링크](https://ops.fhwa.dot.gov/trafficanalysistools/tat_vol3/sectapp_a.htm)
- 미시 모델은 개별 차량 궤적을 내지만 보정할 파라미터가 많고, 중시 모델은 개별 차량을 집계 관계(속도-교통량)로 움직이며, 거시 모델은 혼잡의 범위를 보는 데 적합. 이벤트 방식은 상태 변화가 드문 시스템에 적합.
- 쓰인 곳: 1단계 교통 모델 선택(ADR 0004).

## 데이터 라이선스

<a id="kogl"></a>
**공공누리 제1유형** [문화체육관광부](https://www.mcst.go.kr/site/s_open/kogl/koglType.jsp): 출처 표시, 상업적 이용 가능, 변형·2차적 저작물 작성 가능. 기상청 저작물은 제1유형 표시 대상([기상청 저작권 정책](https://www.kma.go.kr/kma/guide/copyright.jsp)) — 단, 개별 자료에 붙은 유형 표시를 확인할 것.

<a id="cds"></a>
**Copernicus Climate Data Store**: 2025-07-02부터 "Licence to use Copernicus Products"가 CC-BY 4.0으로 대체됨([ECMWF 포럼](https://forum.ecmwf.int/t/cc-by-licence-to-replace-licence-to-use-copernicus-products-on-02-july-2025/13464)). CMIP6 원자료는 파일의 `license` 속성을 따라야 하며 일부 모델은 비상업(CC BY-NC-SA) → 사용 불가([CMIP6 이용 조건](https://ewds.climate.copernicus.eu/licences/cmip6-wps)).

## 지형 데이터 받는 곳

<a id="copernicus-dem-aws"></a>
**Copernicus DEM GLO-30 — AWS Open Data 버킷** (조사일 2026-11-27, 직접 열어 확인)
- 버킷 안내: [copernicus-dem-30m readme](https://copernicus-dem-30m.s3.amazonaws.com/readme.html). Copernicus DEM은 건물·인프라·식생을 포함한 지표면 모델(DSM)이고, Cloud Optimized GeoTIFF로 제공. 바다는 타일이 없고 고도 0으로 간주.
- 경로 규칙: `Copernicus_DSM_COG_[해상도]_[북위]_[동경]_DEM/` 폴더에 같은 이름의 `.tif`. 해상도는 각초 단위(GLO-30은 `10`), 북위·동경은 `N36_00`, `E127_00`처럼 소수부가 항상 `00`인 타일 남서쪽 정수 좌표.
- 실제 확인: [버킷 목록 (prefix N36_00_E127)](https://copernicus-dem-30m.s3.amazonaws.com/?list-type=2&prefix=Copernicus_DSM_COG_10_N36_00_E127&max-keys=10)에 `Copernicus_DSM_COG_10_N36_00_E127_00_DEM/` 폴더가 있고, [타일 파일](https://copernicus-dem-30m.s3.amazonaws.com/Copernicus_DSM_COG_10_N36_00_E127_00_DEM/Copernicus_DSM_COG_10_N36_00_E127_00_DEM.tif)이 HTTP 200, 약 49.6MB.
- 위도 0~50°의 GLO-30 타일은 3600×3600 픽셀(원본 3601에서 동·남쪽 공유 행·열 제거), DEFLATE 압축.
- 라이선스: 버킷 안내는 Copernicus Programme의 이용 조건(무료, 일반 공개)을 가리킴. 표기 문구는 [ATTRIBUTION.md](../../ATTRIBUTION.md)를 따름.
- 쓰인 곳: `gis/src/dongne_gis/terrain.py`, 이슈 #2

## 토지 피복 추정 (도로 폭)

<a id="road-width"></a>
**도로의 구조·시설 기준에 관한 규칙** (국토교통부령 제1588호, 2026-06-03 시행) [국가법령정보센터](https://www.law.go.kr/법령/도로의구조ㆍ시설기준에관한규칙) — 본문을 직접 열어 확인 (조사일 2026-11-27)
- 제10조 ③ 차로의 최소 폭 (도시지역): 설계속도 100 km/h 이상 3.50 m, 80 이상 3.25 m, 70 이상 3.25 m, 60 이상 3.00 m, 60 미만 3.00 m. ④ 설계속도 40 km/h 이하 도시지역 차로는 불가피하면 2.75 m 이상.
- 제16조 ③ 보도의 유효폭은 최소 2 m 이상 (지방지역 도로와 도시지역 국지도로는 불가피하면 1.5 m 이상).
- 쓰인 곳: `gis/src/dongne_gis/landcover.py`. 차로 폭 3.00 m(원도심 도로를 설계속도 80 km/h 미만으로 봄), motorway·trunk 3.25 m, footway 2.0 m. 이 값은 "최소" 폭이므로 실제 포장 면적보다 작게 추정될 수 있음 (보수적).
- **가정 (근거 없음)**: `lanes` 태그가 없을 때 유형별 차로 수, pedestrian 4 m, cycleway·path 1.5 m, track 3 m, 나무 점 수관 반경 3 m, 가로수 줄 폭 6 m, 하천 선 폭(river 20 m, stream 8 m, canal 8 m, drain 2 m, ditch 1.5 m). 측정 자료를 찾으면 교체.

## 도로 그래프 기본값 (1단계 교통)

<a id="road-lanes"></a>
**가정 (근거 없음)** — `core/crates/world/src/road_graph.rs`. 측정·법령 근거를 찾으면 교체합니다.
- `lanes` 태그가 없을 때 양방향 합계 차선 수: motorway·trunk·primary 4, secondary·tertiary·unclassified·residential 2, `*_link`·living_street·service 1. 토지 피복의 도로 폭(`gis/src/dongne_gis/landcover.py`)과 같은 값.
- 양방향 도로는 차선을 반씩 나누고 방향별 최소 1 (1차로 service·living_street도 양방향 통행으로 봄).
- 대기열 용량용 차량 간격 7.5 m (승용차 길이 + 정지 간격).
- 자유 흐름 속도 (km/h): motorway 100, trunk 80, primary 60, secondary·tertiary 50, unclassified·residential 30, living_street·service 20. 맵 패키지에 `maxspeed`가 들어오면 그 값을 우선하도록 바꿀 예정.
- 참고: 차로 **폭**은 법령 근거가 있음 ([road-width](#road-width)). 차로 **수**와 속도는 아직 없음.
