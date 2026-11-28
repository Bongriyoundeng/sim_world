extends Node3D
## 해·하늘·주변광을 시각과 날짜(계절)에 맞춘다.
##
## 해 위치: 맵 원점 위도·경도에서 간단한 태양 위치 식 (적위 + 시간각 + 균시차 근사).
## 근거와 가정: docs/design/research-log.md#sky-light
##
##   set_time(hour, day_of_year)  hour = 한국 표준시(KST, UTC+9) 시각 0~24, day_of_year = 1~365

const KST_MERIDIAN := 135.0   # KST 기준 경선 (UTC+9)

## 계절별 대표일 (각 계절 가운데 달의 15일)과 지형 색 보정
const SEASONS := {
	"spring": {"day": 105, "tint": Color(1.0, 1.0, 0.96)},     # 4월 15일
	"summer": {"day": 196, "tint": Color(0.9, 1.0, 0.88)},     # 7월 15일
	"autumn": {"day": 288, "tint": Color(1.1, 0.95, 0.75)},    # 10월 15일
	"winter": {"day": 15, "tint": Color(0.98, 0.93, 0.86)},    # 1월 15일
}
const SEASON_ORDER := ["spring", "summer", "autumn", "winter"]

var lat_deg := 36.993
var lon_deg := 127.087
var sun: DirectionalLight3D
var moon: DirectionalLight3D   # 밤에 지형·건물 윤곽이 보이도록 하는 약한 푸른 빛 (그림자 없음)
var env: Environment
var sky_mat: ProceduralSkyMaterial
var building_mat: ShaderMaterial
var terrain_mat: StandardMaterial3D
var street: Node3D          # scripts/streetscape.gd: set_season, set_night
var elevation_deg := 0.0
var azimuth_deg := 0.0
var hour := 10.0
var day_of_year := 196


func _init(origin: Vector2 = Vector2(36.993, 127.087)) -> void:
	name = "SkyLight"
	lat_deg = origin.x
	lon_deg = origin.y

	sun = DirectionalLight3D.new()
	sun.name = "Sun"
	sun.shadow_enabled = true
	sun.shadow_bias = 0.05
	sun.shadow_normal_bias = 1.0
	sun.directional_shadow_mode = DirectionalLight3D.SHADOW_PARALLEL_4_SPLITS
	sun.directional_shadow_max_distance = 3000.0
	sun.directional_shadow_blend_splits = true
	add_child(sun)
	moon = DirectionalLight3D.new()
	moon.name = "Moon"
	moon.rotation_degrees = Vector3(-50, -30, 0)
	moon.light_color = Color(0.55, 0.65, 0.9)
	moon.shadow_enabled = false
	add_child(moon)

	sky_mat = ProceduralSkyMaterial.new()
	var sky := Sky.new()
	sky.sky_material = sky_mat
	env = Environment.new()
	env.background_mode = Environment.BG_SKY
	env.sky = sky
	env.ambient_light_source = Environment.AMBIENT_SOURCE_SKY
	env.reflected_light_source = Environment.REFLECTION_SOURCE_SKY
	env.tonemap_mode = Environment.TONE_MAPPER_FILMIC
	# 건물 사이·벽 아래 접지 그늘
	env.ssao_enabled = true
	env.ssao_radius = 3.0
	env.ssao_intensity = 1.6
	# 먼 곳 공기 원근
	env.fog_enabled = true
	env.fog_mode = Environment.FOG_MODE_DEPTH
	env.fog_depth_begin = 900.0
	env.fog_depth_end = 4500.0
	env.fog_density = 0.2
	# 밤 창 불빛·간판이 조금 번지게 (밝은 발광만)
	env.glow_enabled = true
	env.glow_hdr_threshold = 1.5
	env.glow_intensity = 0.6
	env.glow_bloom = 0.0
	var we := WorldEnvironment.new()
	we.environment = env
	add_child(we)


## 계절 이름 → 대표일
static func season_day(season: String) -> int:
	return SEASONS.get(season, SEASONS["summer"])["day"]


func set_season(season: String) -> void:
	day_of_year = season_day(season)
	if terrain_mat:
		terrain_mat.albedo_color = SEASONS.get(season, SEASONS["summer"])["tint"]
	if street:
		street.set_season(season if SEASONS.has(season) else "summer")
	set_time(hour, day_of_year)


func set_time(h: float, doy: int) -> void:
	hour = fposmod(h, 24.0)
	day_of_year = doy
	_sun_position()
	_apply()


## 간단한 태양 위치 (오차 약 1° 이내면 표시에 충분).
func _sun_position() -> void:
	var g := TAU / 365.0 * (day_of_year - 1 + (hour - 12.0) / 24.0)
	var decl := 0.006918 - 0.399912 * cos(g) + 0.070257 * sin(g) - 0.006758 * cos(2 * g) \
		+ 0.000907 * sin(2 * g) - 0.002697 * cos(3 * g) + 0.00148 * sin(3 * g)
	var eot_min := 229.18 * (0.000075 + 0.001868 * cos(g) - 0.032077 * sin(g) - 0.014615 * cos(2 * g) - 0.040849 * sin(2 * g))
	var solar_min := hour * 60.0 + eot_min + 4.0 * (lon_deg - KST_MERIDIAN)
	var ha := deg_to_rad(solar_min / 4.0 - 180.0)
	var lat := deg_to_rad(lat_deg)
	var sin_el := sin(lat) * sin(decl) + cos(lat) * cos(decl) * cos(ha)
	var el := asin(clampf(sin_el, -1.0, 1.0))
	var cos_az := (sin(decl) - sin(el) * sin(lat)) / maxf(cos(el) * cos(lat), 1e-6)
	var az := acos(clampf(cos_az, -1.0, 1.0))
	if ha > 0.0:
		az = TAU - az
	elevation_deg = rad_to_deg(el)
	azimuth_deg = rad_to_deg(az)


func _apply() -> void:
	var el := elevation_deg
	var az := deg_to_rad(azimuth_deg)
	var e := deg_to_rad(maxf(el, 2.0))   # 지평선 아래면 빛을 끄되 방향은 지평선 근처로
	# 해 쪽 방향: 로컬 (동, 북, 위) → Godot (x, 위, -북)
	var to_sun := Vector3(cos(e) * sin(az), sin(e), -cos(e) * cos(az))
	sun.basis = Basis.looking_at(-to_sun, Vector3.UP if absf(to_sun.y) < 0.99 else Vector3.FORWARD)

	var day := smoothstep(-6.0, 10.0, el)          # 0 밤 ~ 1 낮
	var low := 1.0 - smoothstep(0.0, 20.0, el)      # 해가 낮을수록 1 (노을)
	var twilight := smoothstep(-8.0, 0.0, el) * (1.0 - smoothstep(4.0, 16.0, el))

	sun.visible = el > -1.0
	sun.light_energy = 1.25 * smoothstep(-1.0, 12.0, el)
	sun.light_color = Color(1.0, 0.96, 0.9).lerp(Color(1.0, 0.6, 0.35), low)

	var day_top := Color(0.36, 0.55, 0.80)
	var day_hor := Color(0.72, 0.80, 0.88)
	var night_top := Color(0.015, 0.02, 0.05)
	var night_hor := Color(0.05, 0.06, 0.10)
	var dusk_hor := Color(0.95, 0.55, 0.35)
	sky_mat.sky_top_color = night_top.lerp(day_top, day)
	var hor := night_hor.lerp(day_hor, day).lerp(dusk_hor, twilight * 0.8)
	sky_mat.sky_horizon_color = hor
	sky_mat.ground_horizon_color = hor
	sky_mat.ground_bottom_color = Color(0.05, 0.05, 0.06).lerp(Color(0.40, 0.42, 0.40), day)
	sky_mat.sun_angle_max = 30.0

	moon.light_energy = 0.12 * (1.0 - day)
	env.ambient_light_energy = lerpf(0.9, 0.45, day)   # 밤에는 어두운 하늘빛을 조금 더 받음
	env.tonemap_exposure = lerpf(1.6, 1.0, day)
	env.fog_light_color = hor

	var night := 1.0 - smoothstep(-4.0, 6.0, el)
	if building_mat:
		building_mat.set_shader_parameter("night", night)
	if street:
		street.set_night(night)


func describe() -> String:
	return "%02d:%02d 해 고도 %.0f° 방위 %.0f°" % [int(hour), int(fposmod(hour * 60.0, 60.0)), elevation_deg, azimuth_deg]
