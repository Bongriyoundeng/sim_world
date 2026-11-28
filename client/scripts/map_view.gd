extends Node3D
## 맵 패키지 표시: 지형 메시, 도로 띠, 건물 상자.
##
## 코어(DongneWorld, GDExtension)가 맵 패키지를 읽고, 이 스크립트는 그 데이터로 메시를 만든다.
## 좌표: 맵 패키지 로컬 미터 (x 동쪽, y 북쪽) → Godot (x, 높이, -y).
##
## 명령줄 (godot --path client -- …):
##   --map <폴더>        맵 패키지 (기본: 저장소의 maps/pyeongtaek-station)
##   --quit-after-load   메시를 다 만들고 종료 (헤드리스 확인용)
##   --capture <png>     몇 프레임 그린 뒤 화면을 저장하고 종료
##   --view <이름>       처음 카메라 위치: overview(기본), street(원도심 저층), apartments(아파트 단지), boulevard(큰길 가로수)
##   --time <HH:MM>      조명 시각 (KST, 기본 09:00). 시뮬레이션이 돌면 시뮬레이션 시각을 따름 (L 키로 전환)
##   --season <이름>     spring·summer(기본)·autumn·winter: 해 길과 지형 색
##   그 밖의 옵션(--selftest, --demo)과 조작법은 scripts/sim_controller.gd

const DEFAULT_MAP := "../maps/pyeongtaek-station"
const ROAD_LIFT_M := 0.6        # 도로를 지형 위로 살짝 띄움
const LANE_WIDTH_M := 3.0       # 표시용 차로 폭 (research-log.md#road-width)

## 카메라 시작 위치 (눈 위치, 보는 곳). 로컬 좌표 (x, 높이, -y)
const VIEWS := {
	"overview": [Vector3(-420, 980, 1150), Vector3(0, 0, -60)],
	"street": [Vector3(-330, 75, -250), Vector3(-225, 10, -375)],
	"apartments": [Vector3(200, 150, 640), Vector3(380, 15, 380)],
	"boulevard": [Vector3(70, 42, -185), Vector3(-200, 4, -272)],
}

const ROAD_COLORS := {
	"primary": Color(0.95, 0.75, 0.35), "secondary": Color(0.95, 0.85, 0.45),
	"tertiary": Color(0.92, 0.92, 0.75), "residential": Color(0.85, 0.85, 0.85),
	"service": Color(0.7, 0.7, 0.7),
}

var world: RefCounted
var rows := 0
var cols := 0
var res_m := 30.0
var extent := Vector2.ZERO
var heights := PackedFloat32Array()
var cam: Camera3D
var sky: Node3D             # scripts/sky_light.gd
var buildings_node: MeshInstance3D
var street: Node3D          # scripts/streetscape.gd (가로수·가로등)
var terrain_node: MeshInstance3D
var origin := Vector2(36.993, 127.087)   # 위도, 경도 (manifest.json)
var season := "summer"
var start_hour := 9.0


func _ready() -> void:
	var args := OS.get_cmdline_user_args()
	var map_dir := _arg(args, "--map", ProjectSettings.globalize_path("res://").path_join(DEFAULT_MAP)).simplify_path()

	if not ClassDB.class_exists("DongneWorld"):
		_fail("DongneWorld 클래스가 없음: python tools/build_godot_addon.py 로 GDExtension을 빌드하세요")
		return
	world = ClassDB.instantiate("DongneWorld")
	if not world.load_map(map_dir):
		_fail(world.last_error())
		return

	extent = world.extent()
	res_m = world.terrain_resolution()
	if not _load_terrain(map_dir.path_join(world.terrain_file())):
		return
	origin = _read_origin(map_dir, origin)
	season = _arg(args, "--season", "summer")
	start_hour = _parse_hour(_arg(args, "--time", "09:00"))
	terrain_node = _terrain_mesh()
	add_child(terrain_node)
	rebuild_roads()
	buildings_node = _building_mesh()
	add_child(buildings_node)
	street = load("res://scripts/streetscape.gd").new()
	add_child(street)
	street.build(world, height_at)
	_add_camera_and_light(_arg(args, "--view", "overview"))

	print("MAP_VIEW_OK map=%s terrain=%dx%d roads=%d buildings=%d" % [
		world.map_name(), cols, rows, world.road_count(), world.building_count()])

	if "--quit-after-load" in args:
		get_tree().quit(0)
	elif "--capture" in args:
		await _capture(_arg(args, "--capture", "map_view.png"))
	else:
		# 시뮬레이션 진행·차량 표시·도구 (scripts/sim_controller.gd)
		var ctrl: Node = load("res://scripts/sim_controller.gd").new()
		ctrl.name = "SimController"
		add_child(ctrl)


## 도로 메시를 다시 만든다 (도로 추가·삭제 명령 뒤).
func rebuild_roads() -> void:
	var old := get_node_or_null("Roads")
	if old:
		remove_child(old)
		old.queue_free()
	add_child(_road_mesh())
	if street:
		street.build(world, height_at)


func _read_origin(map_dir: String, fallback: Vector2) -> Vector2:
	var txt := FileAccess.get_file_as_string(map_dir.path_join("manifest.json"))
	var m = JSON.parse_string(txt) if txt != "" else null
	if m is Dictionary and m.has("origin"):
		return Vector2(float(m["origin"]["lat"]), float(m["origin"]["lon"]))
	return fallback


static func _parse_hour(t: String) -> float:
	var parts := t.split(":")
	return float(parts[0]) + (float(parts[1]) / 60.0 if parts.size() > 1 else 0.0)


func _arg(args: PackedStringArray, name: String, default: String) -> String:
	var i := args.find(name)
	return args[i + 1] if i >= 0 and i + 1 < args.size() else default


func _fail(msg: String) -> void:
	push_error(msg)
	printerr("MAP_VIEW_FAIL ", msg)
	get_tree().quit(1)


# ── 지형 ──────────────────────────────────────────────

func _load_terrain(path: String) -> bool:
	var img := Image.load_from_file(path)
	if img == null or img.is_empty():
		_fail("높이맵을 읽을 수 없음: " + path)
		return false
	cols = img.get_width()
	rows = img.get_height()
	var r: Vector2 = world.terrain_range()
	heights.resize(rows * cols)
	for y in rows:
		for x in cols:
			# Godot는 16비트 PNG를 8비트로 읽는다 → 약 (최고−최저)/255 m 단위. 표시에는 충분하다.
			heights[y * cols + x] = r.x + img.get_pixel(x, y).r * (r.y - r.x)
	return true


func _cell_center(row: int, col: int) -> Vector2:
	return Vector2(-extent.x / 2 + (col + 0.5) * res_m, extent.y / 2 - (row + 0.5) * res_m)


## 로컬 좌표 (x, y)의 지형 높이 (셀 중심 사이 쌍선형 보간).
func height_at(p: Vector2) -> float:
	var fc := clampf((p.x + extent.x / 2) / res_m - 0.5, 0, cols - 1)
	var fr := clampf((extent.y / 2 - p.y) / res_m - 0.5, 0, rows - 1)
	var c0 := int(fc)
	var r0 := int(fr)
	var c1 := mini(c0 + 1, cols - 1)
	var r1 := mini(r0 + 1, rows - 1)
	var tx := fc - c0
	var ty := fr - r0
	var top := lerpf(heights[r0 * cols + c0], heights[r0 * cols + c1], tx)
	var bottom := lerpf(heights[r1 * cols + c0], heights[r1 * cols + c1], tx)
	return lerpf(top, bottom, ty)


func _to3(p: Vector2, h: float) -> Vector3:
	return Vector3(p.x, h, -p.y)


func _terrain_mesh() -> MeshInstance3D:
	var r: Vector2 = world.terrain_range()
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for row in rows - 1:
		for col in cols - 1:
			var quad := [Vector2i(row, col), Vector2i(row, col + 1), Vector2i(row + 1, col + 1), Vector2i(row + 1, col)]
			for k in [0, 1, 2, 0, 2, 3]:
				var rc: Vector2i = quad[k]
				var h := heights[rc.x * cols + rc.y]
				var t := inverse_lerp(r.x, r.y, h) if r.y > r.x else 0.0
				st.set_color(Color(0.35, 0.55, 0.3).lerp(Color(0.75, 0.68, 0.5), t))
				st.add_vertex(_to3(_cell_center(rc.x, rc.y), h))
	st.generate_normals()
	var mi := MeshInstance3D.new()
	mi.name = "Terrain"
	mi.mesh = st.commit()
	mi.material_override = _material(Color.WHITE, true)
	return mi


# ── 도로 ──────────────────────────────────────────────

func _road_mesh() -> MeshInstance3D:
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	for e in world.road_count():
		if world.road_removed(e):
			continue
		var pts: PackedVector2Array = world.road_points(e)
		var half := maxf(world.road_lanes(e), 1) * LANE_WIDTH_M / 2
		var hw: String = world.road_highway(e).trim_suffix("_link")
		st.set_color(ROAD_COLORS.get(hw, Color(0.8, 0.8, 0.8)))
		for i in pts.size() - 1:
			var a := pts[i]
			var b := pts[i + 1]
			var d := (b - a).normalized()
			if d == Vector2.ZERO:
				continue
			var n := Vector2(-d.y, d.x) * half
			var ha := height_at(a) + ROAD_LIFT_M
			var hb := height_at(b) + ROAD_LIFT_M
			var q := [_to3(a + n, ha), _to3(b + n, hb), _to3(b - n, hb), _to3(a - n, ha)]
			for k in [0, 1, 2, 0, 2, 3]:
				st.set_normal(Vector3.UP)
				st.add_vertex(q[k])
	var mi := MeshInstance3D.new()
	mi.name = "Roads"
	mi.mesh = st.commit()
	mi.material_override = _material(Color.WHITE, true)
	return mi


# ── 건물 ──────────────────────────────────────────────

## 건물: 용도별 색·창문·지붕 (scripts/buildings.gd, shaders/building.gdshader)
func _building_mesh() -> MeshInstance3D:
	var b: RefCounted = load("res://scripts/buildings.gd").new()
	return b.build(world, height_at)


func _material(albedo: Color, vertex_color: bool) -> StandardMaterial3D:
	var m := StandardMaterial3D.new()
	m.albedo_color = albedo
	m.vertex_color_use_as_albedo = vertex_color
	m.vertex_color_is_srgb = true
	m.cull_mode = BaseMaterial3D.CULL_DISABLED
	m.roughness = 0.9
	return m


# ── 카메라·조명 ────────────────────────────────────────

func _add_camera_and_light(view: String) -> void:
	cam = Camera3D.new()
	cam.name = "Camera"
	cam.far = 6000
	cam.fov = 50
	add_child(cam)
	var v: Array = VIEWS.get(view, VIEWS["overview"])
	cam.look_at_from_position(v[0], v[1], Vector3.UP)

	# 해·하늘: 시각·계절에 따라 (scripts/sky_light.gd)
	sky = load("res://scripts/sky_light.gd").new(origin)
	sky.building_mat = buildings_node.material_override
	sky.terrain_mat = terrain_node.material_override
	sky.street = street
	add_child(sky)
	sky.set_season(season)
	sky.set_time(start_hour, sky.day_of_year)


func _capture(path: String) -> void:
	for i in 10:
		await RenderingServer.frame_post_draw
	var img := get_viewport().get_texture().get_image()
	var err := img.save_png(path)
	print("MAP_VIEW_CAPTURE %s %s" % [path, "ok" if err == OK else "error %d" % err])
	get_tree().quit(0 if err == OK else 1)
