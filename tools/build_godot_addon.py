"""코어 GDExtension(core/crates/godot)을 빌드해 client/addons/dongne_core/bin/에 복사한다.

    python tools/build_godot_addon.py            # release
    python tools/build_godot_addon.py --debug

복사한 라이브러리는 커밋하지 않는다 (.gitignore).
"""

from __future__ import annotations

import argparse
import json
import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CORE = ROOT / "core"
BIN = ROOT / "client" / "addons" / "dongne_core" / "bin"


def main() -> int:
    p = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    p.add_argument("--debug", action="store_true", help="debug 빌드 (기본 release)")
    args = p.parse_args()

    cmd = ["cargo", "build", "-p", "dongne-godot", "--message-format=json-render-diagnostics"]
    if not args.debug:
        cmd.append("--release")
    proc = subprocess.run(cmd, cwd=CORE, stdout=subprocess.PIPE, text=True, encoding="utf-8")
    if proc.returncode != 0:
        return proc.returncode

    libs: list[Path] = []
    for line in proc.stdout.splitlines():
        msg = json.loads(line)
        if msg.get("reason") == "compiler-artifact" and msg["target"]["name"] == "dongne_godot":
            libs += [Path(f) for f in msg["filenames"] if f.endswith((".dll", ".so", ".dylib"))]
    if not libs:
        print("dongne_godot 라이브러리를 찾지 못함", file=sys.stderr)
        return 1

    BIN.mkdir(parents=True, exist_ok=True)
    for lib in libs:
        shutil.copy2(lib, BIN / lib.name)
        print(f"{lib} → {BIN / lib.name}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
