shader_type spatial;
render_mode cull_back, diffuse_burley, specular_schlick_ggx;
// 건물 벽·지붕 셰이더: 층(3 m)과 창 간격(bay)으로 창문·상점 띠·간판을 그린다.
// 텍스처 파일 없이 절차적으로 그린다 (에셋 라이선스 부담 없음).
//
// 정점 데이터 (scripts/buildings.gd):
//   COLOR.rgb  바탕색 (sRGB),  COLOR.a  건물마다 고정된 씨앗 0~1
//   UV         벽: (창 간격 단위 좌표, 건물 바닥에서 높이 m)
//   UV2        (표시 분류, 벽 꼭대기 높이 m) — 그 위는 난간(파라펫)
// 분류: 0 지붕·난간 안쪽, 1 아파트, 2 상가주택, 3 단독주택, 4 상업, 5 공공·학교,
//       6 공장·창고, 7 역, 8 창 없는 벽

uniform float storey_m = 3.0;
// 0 = 낮, 1 = 밤 (창 불빛·간판 발광). 시간·계절 조명 단계에서 쓴다.
uniform float night = 0.0;
uniform vec3 glass_color : source_color = vec3(0.30, 0.38, 0.46);
uniform vec3 glass_light : source_color = vec3(1.0, 0.80, 0.52);

varying vec3 wpos;

void vertex() {
	wpos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
}

float hash12(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

vec3 to_linear(vec3 c) {
	return pow(c, vec3(2.2));
}

// 셀 좌표 c (0~1) 안의 사각형 [lo, hi]: 안이면 1. aa는 경계 흐림 폭.
float rect(vec2 c, vec2 lo, vec2 hi, vec2 aa) {
	vec2 a = smoothstep(lo - aa, lo + aa, c) * (1.0 - smoothstep(hi - aa, hi + aa, c));
	return a.x * a.y;
}

vec3 sign_color(float h) {
	// 간판 색: 빨강, 노랑, 파랑, 흰색, 초록, 주황
	if (h < 0.18) return vec3(0.80, 0.16, 0.14);
	if (h < 0.36) return vec3(0.95, 0.80, 0.20);
	if (h < 0.52) return vec3(0.16, 0.36, 0.75);
	if (h < 0.70) return vec3(0.92, 0.92, 0.90);
	if (h < 0.85) return vec3(0.15, 0.55, 0.32);
	return vec3(0.95, 0.50, 0.15);
}

void fragment() {
	int style = int(UV2.x + 0.5);
	vec3 base = to_linear(COLOR.rgb);
	float seed = COLOR.a;
	ROUGHNESS = 0.88;
	SPECULAR = 0.3;

	if (style == 0) {
		// 지붕: 2 m 격자로 얼룩을 조금 넣는다
		float n = hash12(floor(wpos.xz * 0.5) + seed * 97.0);
		ALBEDO = base * (0.93 + 0.1 * n);
	} else {
		float u = UV.x;
		float v = UV.y;
		float wall_top = UV2.y;
		// 땅 가까이 조금 어둡게 (빗물 얼룩·접지)
		vec3 wall = base * mix(0.78, 1.0, smoothstep(0.0, 1.2, v));
		if (v > wall_top) {
			ALBEDO = base * 0.92;   // 난간 바깥면
		} else if (style == 8) {
			ALBEDO = wall;
		} else {
			float fl = floor(v / storey_m);
			float fv = fract(v / storey_m);
			float bay = floor(u);
			vec2 c = vec2(fract(u), fv);
			// 멀리서는 무늬가 깨지므로 평균색으로 흐린다
			vec2 fw = vec2(fwidth(u), fwidth(v / storey_m));
			vec2 aa = clamp(fw, vec2(0.004), vec2(0.5));
			float far = smoothstep(0.12, 0.35, max(fw.x, fw.y));
			float id = hash12(vec2(bay + seed * 1013.0, fl + seed * 71.0));

			float glass = 0.0;      // 유리 (창)
			float shop = 0.0;       // 1층 상점 유리
			float signage = 0.0;    // 간판 띠
			float band = 0.0;       // 층 사이 띠 (밝게)
			bool ground = fl < 0.5;
			bool top_storey = v > wall_top - storey_m;

			if (style == 1) {
				// 아파트: 칸마다 넓은 발코니 창, 가운데 창틀, 층마다 슬래브 띠
				glass = rect(c, vec2(0.07, 0.30), vec2(0.93, 0.90), aa)
					* (1.0 - rect(c, vec2(0.485, 0.30), vec2(0.515, 0.90), aa));
				band = 1.0 - smoothstep(0.08, 0.08 + aa.y, fv);
			} else if (style == 2 || style == 4) {
				if (ground) {
					shop = rect(c, vec2(0.05, 0.03), vec2(0.95, 0.70), aa);
					signage = rect(c, vec2(0.02, 0.76), vec2(0.98, 0.96), aa);
				} else if (style == 2) {
					glass = rect(c, vec2(0.22, 0.32), vec2(0.78, 0.82), aa);
					// 위층 일부 칸에도 간판 (2층 상가)
					if (fl < 1.5 && id > 0.6) {
						signage = rect(c, vec2(0.06, 0.06), vec2(0.94, 0.26), aa);
					}
				} else {
					// 상업: 띠창 + 칸 경계 창틀
					glass = rect(c, vec2(0.0, 0.22), vec2(1.0, 0.88), aa)
						* (1.0 - rect(c, vec2(-0.03, 0.22), vec2(0.03, 0.88), aa));
				}
			} else if (style == 3) {
				// 단독주택: 칸마다 있거나 없는 작은 창
				if (id > 0.3) {
					glass = rect(c, vec2(0.28, 0.36), vec2(0.72, 0.78), aa);
				}
			} else if (style == 5) {
				glass = rect(c, vec2(0.12, 0.30), vec2(0.88, 0.86), aa);
				band = 1.0 - smoothstep(0.10, 0.10 + aa.y, fv);
			} else if (style == 6) {
				// 공장·창고: 세로 골판 무늬 + 꼭대기 띠창
				float rib = smoothstep(0.4, 0.5, abs(fract(u * 6.0) - 0.5));
				wall *= mix(1.0, 0.86, rib);
				if (top_storey) {
					glass = rect(c, vec2(0.06, 0.40), vec2(0.94, 0.80), aa);
				}
			} else if (style == 7) {
				// 역사: 큰 유리면
				glass = rect(c, vec2(0.04, 0.08), vec2(0.96, 0.94), aa);
			}

			// 유리: 칸마다 밝기를 조금 다르게 (커튼·반사)
			vec3 g = to_linear(glass_color) * (0.75 + 0.5 * id);
			vec3 sc = to_linear(sign_color(hash12(vec2(bay * 3.1 + seed * 59.0, fl))));
			vec3 col = wall;
			col = mix(col, base * 1.12, band);
			col = mix(col, g, glass);
			col = mix(col, to_linear(vec3(0.12, 0.15, 0.17)), shop);
			col = mix(col, sc, signage);

			// 멀리서 보이는 평균색
			float cover = style == 1 ? 0.45 : (style == 4 || style == 7) ? 0.55 : style == 3 ? 0.15 : 0.25;
			vec3 avg = mix(wall, g, cover);
			if (style == 2 || style == 4) {
				avg = mix(avg, mix(to_linear(vec3(0.12, 0.15, 0.17)), sc, 0.3), ground ? 0.7 : 0.0);
			}
			ALBEDO = mix(col, avg, far);

			float glassy = max(glass, shop) * (1.0 - far);
			ROUGHNESS = mix(0.88, 0.12, glassy);
			SPECULAR = mix(0.3, 0.7, glassy);
			// 밤: 칸의 일부만 불이 켜짐, 간판은 모두
			vec3 lit = to_linear(glass_light) * (step(0.45, id) * glass + shop) + sc * signage;
			EMISSION = night * lit * 2.0;
		}
	}
}
