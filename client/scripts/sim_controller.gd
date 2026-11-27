extends Node
## 시뮬레이션 진행, 차량 보간 표시, 도로 그리기·삭제, 구역 칠하기, HUD, 카메라 조작.
##
## 조작
##   1 보기  2 도로 그리기(두 번 클릭)  3 도로 삭제(클릭)
##   4 주거  5 상업  6 업무  7 공원  8 구역 지우기  (끌어서 칠하기)
##   Space 멈춤/재생   + / - 속도   C 출퇴근 통행 만들기   T 무작위 차량 200대
##   W A S D 이동   휠 확대·축소   오른쪽 버튼 끌기 회전   Esc 도로 그리기 취소
##
## 명령줄 (godot --path client -- …)
##   --selftest               명령·차량 위치·보간 경로를 확인하고 종료 (헤드리스 가능)
##   --demo <폴더>            출퇴근 시나리오를 돌리며 프레임 PNG를 저장 (tools/make_demo_gif.py)
##   --demo-frames <n>        저장할 프레임 수 (기본 180)

enum Tool { VIEW, ROAD, REMOVE, ZONE }

const ZONE_NAMES := ["residential", "commercial", "office", "park"]
const ZONE_LABELS := ["주거", "상업", "업무", "공원"]
const ZONE_COLORS := [
	Color(0.35, 0.85, 0.35, 0.45), Color(0.3, 0.55, 1.0, 0.45),
	Color(1.0, 0.6, 0.2, 0.45), Color(0.1, 0.55, 0.2, 0.45),
]
const CHUNK_TICKS := 5                     # 0.5초씩 진행하고 그 사이를 보간
const SPEEDS := [0.0, 1.0, 5.0, 20.0, 60.0, 120.0, 300.0]   # 실시간 1초당 시뮬레이션 초
const MAX_CHUNKS_PER_FRAME := 120
const VEHICLE_SIZE := Vector3(6.0, 2.5, 3.0)   # 표시용으로 조금 키운 차량 (길이, 높이, 폭)

var view: Node3D
var world: RefCounted
var speed_i := 4
var acc := 0.0
var prev := {}          # 차량 id → Vector3(x, y, 진행 방향)
var cur := {}
var tool := Tool.VIEW
var zone_i := 0         # -1 = 구역 지우기
var road_start = null   # Vector2 또는 null
var painting := false
var paint_cells := {}
var pending_cmds := 0
var n_outcomes := 0
var message := ""

var vehicles_mm: MultiMeshInstance3D
var zones_node: MeshInstance3D
var marker: MeshInstance3D
var hud: Label

var cam_target := Vector3(0, 0, -60)
var cam_dist := 1610.0
var cam_yaw := -19.0
var cam_pitch := 37.0

var demo_dir := ""
var demo_frames := 0
var demo_total := 180


func _ready() -> void:
	view = get_parent()
	world = view.world
	_make_vehicle_layer()
	_make_marker()
	_make_hud()
	rebuild_zones()
	_apply_camera()

	var args := OS.get_cmdline_user_args()
	if "--selftest" in args:
		_selftest()
	elif "--demo" in args:
		demo_dir = view._arg(args, "--demo", "demo_frames")
		demo_total = int(view._arg(args, "--demo-frames", "180"))
		_start_demo()


# ── 진행과 보간 ──────────────────────────────────────────

func _process(delta: float) -> void:
	_camera_keys(delta)
	var chunk_s := CHUNK_TICKS * 0.1
	var speed: float = SPEEDS[speed_i]
	if demo_dir != "":
		delta = 1.0 / 30.0          # 데모는 고정 프레임 간격 (결정론적 영상)
	if speed > 0.0:
		acc += delta * speed
		var n := 0
		while acc >= chunk_s and n < MAX_CHUNKS_PER_FRAME:
			_advance()
			acc -= chunk_s
			n += 1
		if n == MAX_CHUNKS_PER_FRAME:
			acc = 0.0
	_draw_vehicles(clampf(acc / chunk_s, 0.0, 1.0))
	_update_hud()
	if demo_dir != "":
		_demo_frame()


func _advance() -> void:
	prev = cur
	world.step(CHUNK_TICKS)
	cur = _fetch_positions()
	if pending_cmds > 0:
		_check_outcomes()


func _fetch_positions() -> Dictionary:
	var out := {}
	var a: PackedFloat32Array = world.vehicle_positions()
	for i in range(0, a.size(), 4):
		out[int(a[i])] = Vector3(a[i + 1], a[i + 2], a[i + 3])
	return out


func _draw_vehicles(alpha: float) -> void:
	var mm := vehicles_mm.multimesh
	mm.instance_count = cur.size()
	var i := 0
	for id in cur:
		var c: Vector3 = cur[id]
		var p: Vector3 = prev.get(id, c)
		var xy := Vector2(lerpf(p.x, c.x, alpha), lerpf(p.y, c.y, alpha))
		var heading := lerp_angle(p.z, c.z, alpha)
		var pos := Vector3(xy.x, view.height_at(xy) + 0.6 + VEHICLE_SIZE.y / 2, -xy.y)
		mm.set_instance_transform(i, Transform3D(Basis(Vector3.UP, heading), pos))
		i += 1


# ── 명령 ──────────────────────────────────────────────

func submit(cmd: Dictionary) -> int:
	var id: int = world.submit(JSON.stringify(cmd))
	if id < 0:
		message = "명령 오류: " + world.last_error()
		return id
	pending_cmds += 1
	world.step(0)       # 멈춰 있어도 지금 틱 경계에서 바로 적용
	_check_outcomes()
	return id


func _check_outcomes() -> void:
	var outs: Array = JSON.parse_string(world.outcomes_json())
	var roads_changed := false
	var zones_changed := false
	for k in range(n_outcomes, outs.size()):
		var r: Dictionary = outs[k]["result"]
		if r["status"] == "applied":
			roads_changed = roads_changed or r.has("edge")
			zones_changed = zones_changed or r.has("cells")
			message = "적용: %s" % JSON.stringify(r)
		else:
			message = "거부: %s" % r.get("reason", "")
	n_outcomes = outs.size()
	pending_cmds = 0
	if roads_changed:
		view.rebuild_roads()
	if zones_changed:
		rebuild_zones()


# ── 입력 ──────────────────────────────────────────────

func _unhandled_input(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and not event.echo:
		_key(event.keycode)
	elif event is InputEventMouseButton:
		_mouse_button(event)
	elif event is InputEventMouseMotion:
		if event.button_mask & MOUSE_BUTTON_MASK_RIGHT:
			cam_yaw -= event.relative.x * 0.3
			cam_pitch = clampf(cam_pitch + event.relative.y * 0.3, 10, 89)
			_apply_camera()
		elif painting:
			_paint(event.position)


func _key(k: Key) -> void:
	match k:
		KEY_1: _set_tool(Tool.VIEW)
		KEY_2: _set_tool(Tool.ROAD)
		KEY_3: _set_tool(Tool.REMOVE)
		KEY_4, KEY_5, KEY_6, KEY_7:
			zone_i = k - KEY_4
			_set_tool(Tool.ZONE)
		KEY_8:
			zone_i = -1
			_set_tool(Tool.ZONE)
		KEY_SPACE:
			speed_i = 4 if speed_i == 0 else 0
		KEY_EQUAL, KEY_KP_ADD:
			speed_i = mini(speed_i + 1, SPEEDS.size() - 1)
		KEY_MINUS, KEY_KP_SUBTRACT:
			speed_i = maxi(speed_i - 1, 1)
		KEY_C:
			message = "출퇴근 통행: " + world.add_commute(2026)
		KEY_T:
			message = "무작위 차량 %d대" % world.add_random_trips(200, Time.get_ticks_msec(), 120.0)
		KEY_ESCAPE:
			road_start = null
			marker.visible = false


func _set_tool(t: Tool) -> void:
	tool = t
	road_start = null
	marker.visible = false


func _mouse_button(e: InputEventMouseButton) -> void:
	if e.button_index == MOUSE_BUTTON_WHEEL_UP and e.pressed:
		cam_dist = maxf(cam_dist * 0.9, 80)
		_apply_camera()
	elif e.button_index == MOUSE_BUTTON_WHEEL_DOWN and e.pressed:
		cam_dist = minf(cam_dist * 1.1, 4000)
		_apply_camera()
	elif e.button_index == MOUSE_BUTTON_LEFT:
		if not e.pressed:
			if painting:
				_finish_paint()
			return
		var p = pick(e.position)
		if p == null:
			return
		match tool:
			Tool.ROAD:
				if road_start == null:
					road_start = p
					marker.position = Vector3(p.x, view.height_at(p) + 2, -p.y)
					marker.visible = true
				else:
					add_road(road_start, p)
					road_start = null
					marker.visible = false
			Tool.REMOVE:
				var edge: int = world.nearest_edge(p, 20.0)
				if edge >= 0:
					submit({"type": "remove_road", "edge": edge})
				else:
					message = "20 m 안에 도로가 없음"
			Tool.ZONE:
				painting = true
				paint_cells = {}
				_paint(e.position)


func add_road(a: Vector2, b: Vector2) -> int:
	return submit({
		"type": "add_road", "highway": "residential", "lanes": 2, "oneway": false,
		"from": {"point": {"x": a.x, "y": a.y}}, "to": {"point": {"x": b.x, "y": b.y}},
	})


func _paint(screen: Vector2) -> void:
	var p = pick(screen)
	if p == null:
		return
	var cell: Vector2i = world.cell_at(p)
	if cell.x >= 0:
		paint_cells[cell] = true


func _finish_paint() -> void:
	painting = false
	if paint_cells.is_empty():
		return
	var cells := []
	for c in paint_cells:
		cells.append([c.x, c.y])
	submit({"type": "set_zone", "area": {"cells": cells}, "zone": ZONE_NAMES[zone_i] if zone_i >= 0 else null})


## 화면 좌표 → 지형 위 로컬 좌표 (맵 밖이면 null). 높이를 몇 번 고쳐 가며 광선과 지형의 교점을 찾는다.
func pick(screen: Vector2):
	var cam: Camera3D = view.cam
	var o := cam.project_ray_origin(screen)
	var d := cam.project_ray_normal(screen)
	if d.y >= 0:
		return null
	var h := 0.0
	var p := Vector2.ZERO
	for i in 4:
		var hit := o + d * ((h - o.y) / d.y)
		p = Vector2(hit.x, -hit.z)
		h = view.height_at(p)
	if absf(p.x) > view.extent.x / 2 or absf(p.y) > view.extent.y / 2:
		return null
	return p


# ── 카메라 ────────────────────────────────────────────

func _camera_keys(delta: float) -> void:
	var move := Vector2.ZERO
	if Input.is_key_pressed(KEY_W): move.y += 1
	if Input.is_key_pressed(KEY_S): move.y -= 1
	if Input.is_key_pressed(KEY_D): move.x += 1
	if Input.is_key_pressed(KEY_A): move.x -= 1
	if move == Vector2.ZERO:
		return
	var yaw := deg_to_rad(cam_yaw)
	var fwd := Vector3(-sin(yaw), 0, -cos(yaw))
	var right := Vector3(cos(yaw), 0, -sin(yaw))
	cam_target += (fwd * move.y + right * move.x) * cam_dist * 0.6 * delta
	_apply_camera()


func _apply_camera() -> void:
	var yaw := deg_to_rad(cam_yaw)
	var pitch := deg_to_rad(cam_pitch)
	var offset := Vector3(sin(yaw) * cos(pitch), sin(pitch), cos(yaw) * cos(pitch)) * cam_dist
	view.cam.look_at_from_position(cam_target + offset, cam_target, Vector3.UP)


# ── 표시 레이어 ────────────────────────────────────────

func _make_vehicle_layer() -> void:
	var box := BoxMesh.new()
	box.size = VEHICLE_SIZE
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(0.9, 0.15, 0.15)
	mat.emission_enabled = true
	mat.emission = Color(0.5, 0.05, 0.05)
	box.material = mat
	var mm := MultiMesh.new()
	mm.transform_format = MultiMesh.TRANSFORM_3D
	mm.mesh = box
	vehicles_mm = MultiMeshInstance3D.new()
	vehicles_mm.name = "Vehicles"
	vehicles_mm.multimesh = mm
	view.add_child(vehicles_mm)


func _make_marker() -> void:
	var s := SphereMesh.new()
	s.radius = 5
	s.height = 10
	var mat := StandardMaterial3D.new()
	mat.albedo_color = Color(1, 0.9, 0.1)
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	s.material = mat
	marker = MeshInstance3D.new()
	marker.mesh = s
	marker.visible = false
	view.add_child(marker)


func rebuild_zones() -> void:
	if zones_node:
		zones_node.queue_free()
	var cells: PackedInt32Array = world.zone_cells()
	var st := SurfaceTool.new()
	st.begin(Mesh.PRIMITIVE_TRIANGLES)
	var half: float = view.res_m / 2
	for i in range(0, cells.size(), 3):
		var c: Vector2 = view._cell_center(cells[i], cells[i + 1])
		var h: float = view.height_at(c) + 0.4
		st.set_color(ZONE_COLORS[cells[i + 2]])
		var q := [
			Vector3(c.x - half, h, -(c.y + half)), Vector3(c.x + half, h, -(c.y + half)),
			Vector3(c.x + half, h, -(c.y - half)), Vector3(c.x - half, h, -(c.y - half)),
		]
		for k in [0, 1, 2, 0, 2, 3]:
			st.add_vertex(q[k])
	zones_node = MeshInstance3D.new()
	zones_node.name = "Zones"
	if cells.size() > 0:
		zones_node.mesh = st.commit()
	var mat := StandardMaterial3D.new()
	mat.vertex_color_use_as_albedo = true
	mat.transparency = BaseMaterial3D.TRANSPARENCY_ALPHA
	mat.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED
	mat.cull_mode = BaseMaterial3D.CULL_DISABLED
	zones_node.material_override = mat
	view.add_child(zones_node)


func _make_hud() -> void:
	var layer := CanvasLayer.new()
	view.add_child(layer)
	var panel := PanelContainer.new()
	panel.position = Vector2(12, 12)
	layer.add_child(panel)
	hud = Label.new()
	hud.add_theme_font_size_override("font_size", 18)
	panel.add_child(hud)


func _update_hud() -> void:
	var t: float = world.time_s()
	var day := int(t) % 86400
	var tr: Dictionary = JSON.parse_string(world.traffic_json())
	var tool_name: String = ["보기", "도로 그리기", "도로 삭제", "구역 칠하기"][tool]
	if tool == Tool.ZONE:
		tool_name += " (%s)" % (ZONE_LABELS[zone_i] if zone_i >= 0 else "지우기")
	hud.text = "%02d:%02d  ×%d  |  도구: %s\n차량 %d대 · 도로 위 %d대 · 도착 %d · 평균 통근 %.0f초\n%s\n1 보기 2 도로 3 삭제 4~7 구역 8 지우기 · Space 멈춤 · +/- 속도 · C 출퇴근 · T 무작위" % [
		day / 3600, (day % 3600) / 60, int(SPEEDS[speed_i]), tool_name,
		tr["vehicles"], cur.size(), tr["arrived"], tr["mean_travel_s"], message]


# ── 자체 확인 (헤드리스) ───────────────────────────────

func _selftest() -> void:
	var fails := []
	var roads0: int = world.road_count()

	submit({"type": "set_zone", "area": {"cells": [[10, 10], [10, 11]]}, "zone": "residential"})
	if world.zone_cells().size() != 6:
		fails.append("구역 칠하기")
	if zones_node.mesh == null:
		fails.append("구역 표시")

	var id := add_road(Vector2(-700, 700), Vector2(-600, 700))
	if id < 0 or world.road_count() != roads0 + 1:
		fails.append("도로 그리기")

	var edge: int = world.nearest_edge(Vector2(-650, 700), 5.0)
	submit({"type": "remove_road", "edge": edge})
	if not world.road_removed(edge):
		fails.append("도로 삭제")

	if world.submit("{\"type\": \"fly\"}") != -1:
		fails.append("잘못된 명령 거부")

	world.add_random_trips(50, 7, 30.0)
	for i in 40:
		_advance()
	var moved := 0
	for v in cur:
		if prev.has(v) and (prev[v] as Vector3) != (cur[v] as Vector3):
			moved += 1
	if cur.is_empty() or moved == 0:
		fails.append("차량 위치·이동")
	_draw_vehicles(0.5)
	if vehicles_mm.multimesh.instance_count != cur.size():
		fails.append("차량 표시")

	if fails.is_empty():
		print("SELFTEST_OK roads=%d zones=%d vehicles_on_road=%d moved=%d time=%.1fs" % [
			world.road_count(), world.zone_cells().size() / 3, cur.size(), moved, world.time_s()])
		get_tree().quit(0)
	else:
		printerr("SELFTEST_FAIL ", fails)
		get_tree().quit(1)


# ── 데모 녹화 ──────────────────────────────────────────

func _start_demo() -> void:
	DirAccess.make_dir_recursive_absolute(demo_dir)
	var rect := func(x0, y0, x1, y1): return {"polygon": [
		{"x": x0, "y": y0}, {"x": x1, "y": y0}, {"x": x1, "y": y1}, {"x": x0, "y": y1}]}
	# tests/scenarios/scenario.py와 같은 구역
	submit({"type": "set_zone", "area": rect.call(-700, -700, -300, -300), "zone": "residential"})
	submit({"type": "set_zone", "area": rect.call(-250, -400, 50, -100), "zone": "office"})
	submit({"type": "set_zone", "area": rect.call(200, 0, 500, 300), "zone": "commercial"})
	message = "출퇴근 통행: " + world.add_commute(2026)
	world.step(int(7.6 * 3600 * 10))        # 07:36까지 진행
	cur = _fetch_positions()
	prev = cur
	speed_i = 4                              # 실시간 1초 = 시뮬레이션 60초
	cam_target = Vector3(-320, 0, 360)
	cam_dist = 700
	cam_yaw = -25
	cam_pitch = 48
	_apply_camera()


func _demo_frame() -> void:
	if demo_frames >= demo_total:
		return
	await RenderingServer.frame_post_draw
	if demo_frames >= demo_total:
		return
	var img := get_viewport().get_texture().get_image()
	img.save_png(demo_dir.path_join("frame_%04d.png" % demo_frames))
	demo_frames += 1
	if demo_frames >= demo_total:
		print("DEMO_OK frames=%d dir=%s time=%.0fs" % [demo_frames, demo_dir, world.time_s()])
		get_tree().quit(0)
