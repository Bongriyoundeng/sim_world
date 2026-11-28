shader_type spatial;
render_mode cull_back, diffuse_burley, specular_schlick_ggx;
// 건물 벽·지붕 셰이더: 층(3 m)과 창 간격(bay)으로 창문·상점 띠·간판을 그린다.
// 텍스처 파일 없이 절차적으로 그린다 (에셋 라이선스 부담 없음).
//
// 정점 데이터 (scripts/buildings.gd):
//   COLOR.rgb  바탕색 (sRGB),  COLOR.a  건물마다 고정된 씨앗 0~1
//   UV         벽: (창 간격 단위 좌표, 건물 바닥에서 높이 m)
//   UV2        (표시 분류 [+16 = 도로에 면한 벽], 벽 꼭대기 높이 m) — 그 위는 난간(파라펫)
//   CUSTOM0    (동 번호, 벽 길이 m, 0, 0) — 아파트 측벽만. 이때 UV.x는 벽 길이 방향 m
// 분류: 0 지붕·난간 안쪽·부속물, 1 아파트, 2 상가주택, 3 단독주택, 4 상업, 5 공공·학교,
//       6 공장·창고, 7 역, 8 창 없는 벽, 9 돌출 간판

uniform float storey_m = 3.0;
// 0 = 낮, 1 = 밤 (창 불빛·간판 발광). 시간·계절 조명 단계에서 쓴다.
uniform float night = 0.0;
uniform vec3 glass_color : source_color = vec3(0.30, 0.38, 0.46);
uniform vec3 glass_light : source_color = vec3(1.0, 0.80, 0.52);

// 셀 좌표 c (0~1) 안의 사각형 [lo, hi]: 안이면 1. aa는 경계 흐림 폭.
float rect(vec2 c, vec2 lo, vec2 hi, vec2 aa) {
	vec2 a = smoothstep(lo - aa, lo + aa, c) * (1.0 - smoothstep(hi - aa, hi + aa, c));
	return a.x * a.y;
}

varying vec3 wpos;
varying vec2 dong;   // (동 번호, 벽 길이 m)

void vertex() {
	wpos = (MODEL_MATRIX * vec4(VERTEX, 1.0)).xyz;
	dong = CUSTOM0.xy;
}

// 7획 숫자: p는 칸 안 좌표 (0~1, 위가 1). 굵기 t.
float digit(vec2 p, int d, vec2 aa) {
	// 획 a(위) b(오른쪽 위) c(오른쪽 아래) d(아래) e(왼쪽 아래) f(왼쪽 위) g(가운데)
	int masks[10] = int[](0x3F, 0x06, 0x5B, 0x4F, 0x66, 0x6D, 0x7D, 0x07, 0x7F, 0x6F);
	int m = masks[clamp(d, 0, 9)];
	float t = 0.16;
	float ink = 0.0;
	if ((m & 1) != 0) ink = max(ink, rect(p, vec2(0.0, 1.0 - t), vec2(1.0, 1.0), aa));
	if ((m & 2) != 0) ink = max(ink, rect(p, vec2(1.0 - t, 0.5), vec2(1.0, 1.0), aa));
	if ((m & 4) != 0) ink = max(ink, rect(p, vec2(1.0 - t, 0.0), vec2(1.0, 0.5), aa));
	if ((m & 8) != 0) ink = max(ink, rect(p, vec2(0.0, 0.0), vec2(1.0, t), aa));
	if ((m & 16) != 0) ink = max(ink, rect(p, vec2(0.0, 0.0), vec2(t, 0.5), aa));
	if ((m & 32) != 0) ink = max(ink, rect(p, vec2(0.0, 0.5), vec2(t, 1.0), aa));
	if ((m & 64) != 0) ink = max(ink, rect(p, vec2(0.0, 0.5 - t / 2.0), vec2(1.0, 0.5 + t / 2.0), aa));
	return ink;
}

float hash12(vec2 p) {
	vec3 p3 = fract(vec3(p.xyx) * 0.1031);
	p3 += dot(p3, p3.yzx + 33.33);
	return fract((p3.x + p3.y) * p3.z);
}

vec3 to_linear(vec3 c) {
	return pow(c, vec3(2.2));
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
	int raw = int(UV2.x + 0.5);
	bool street = raw >= 16;
	int style = raw % 16;
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
			if (dong.x > 0.5) {
				// 아파트 측벽 동 번호: 벽 위쪽 가운데, 글자 높이 최대 5 m
				float dh = min(5.0, dong.y / 4.0);
				float dw = dh * 0.6;
				float gap = dh * 0.25;
				float nd = dong.x >= 999.5 ? 4.0 : 3.0;
				float total = nd * dw + (nd - 1.0) * gap;
				vec2 p = vec2(u - (dong.y - total) / 2.0, v - (wall_top - 2.0 - dh));
				float k = floor(p.x / (dw + gap));
				float lx = p.x - k * (dw + gap);
				if (p.y >= -0.2 && p.y <= dh + 0.2 && k >= 0.0 && k < nd && lx <= dw + 0.2) {
					int d = int(mod(floor(dong.x / pow(10.0, nd - 1.0 - k) + 0.001), 10.0));
					vec2 aa = clamp(vec2(fwidth(u) / dw, fwidth(v) / dh), vec2(0.01), vec2(0.5));
					float ink = digit(vec2(lx / dw, p.y / dh), d, aa);
					ALBEDO = mix(wall, to_linear(vec3(0.20, 0.26, 0.38)), ink);
				}
			}
		} else if (style == 9) {
			// 돌출 간판: 바탕색에 글자 덩어리 (세로로 쌓임)
			vec2 c = vec2(fract(u / 0.9), fract(v / 0.9));
			float cell = hash12(vec2(floor(u / 0.9), floor(v / 0.9)) + seed * 31.0);
			vec2 aa = clamp(vec2(fwidth(u), fwidth(v)) / 0.9, vec2(0.01), vec2(0.5));
			float glyph = rect(c, vec2(0.18, 0.15), vec2(0.82, 0.85), aa) * step(0.25, cell);
			vec3 ink = dot(base, vec3(0.33)) > 0.5 ? to_linear(vec3(0.75, 0.1, 0.1)) : to_linear(vec3(0.97));
			ALBEDO = mix(base, ink, glyph);
			ROUGHNESS = 0.5;
			EMISSION = night * mix(base, ink, glyph) * 2.0;
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
				// 1층 상점·간판은 도로에 면한 벽에만
				if (ground && street) {
					shop = rect(c, vec2(0.05, 0.03), vec2(0.95, 0.70), aa);
					signage = rect(c, vec2(0.02, 0.76), vec2(0.98, 0.96), aa);
				} else if (style == 2) {
					glass = rect(c, vec2(0.22, 0.32), vec2(0.78, 0.82), aa);
					// 위층 일부 칸에도 간판 (2층 상가)
					if (street && fl < 1.5 && id > 0.6) {
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
			// 간판 글자 덩어리 (칸마다 5자 정도)
			float lc = hash12(vec2(floor(u * 5.0) + seed * 13.0, fl));
			vec2 lp = vec2(fract(u * 5.0), fv);
			float letters = signage * step(0.3, lc) * (1.0 - far)
				* rect(lp, vec2(0.18, fl < 0.5 ? 0.80 : 0.09), vec2(0.82, fl < 0.5 ? 0.92 : 0.23), aa * vec2(5.0, 1.0));
			vec3 lcol = dot(sc, vec3(0.33)) > 0.4 ? to_linear(vec3(0.1)) : to_linear(vec3(0.97));
			col = mix(col, lcol, letters);

			// 멀리서 보이는 평균색
			float cover = style == 1 ? 0.45 : (style == 4 || style == 7) ? 0.55 : style == 3 ? 0.15 : 0.25;
			vec3 avg = mix(wall, g, cover);
			if (style == 2 || style == 4) {
				avg = mix(avg, mix(to_linear(vec3(0.12, 0.15, 0.17)), sc, 0.3), (ground && street) ? 0.7 : 0.0);
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
