# verify.py — 品質ゲート(N-05)。開発時専用(出荷物に Python は含まれない)。
#
# ゲート: fmt / clippy(-D warnings)/ test / wasm リリースビルド + web/ へ配置。
# すべて green で exit 0。ログは .loop/<step>.log に残す。
#
# 使い方: python scripts/verify.py [--fast]  (--fast は wasm ビルドをスキップ)

import shutil
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
MANIFEST = ROOT / "rust" / "Cargo.toml"
WASM_SRC = ROOT / "rust" / "target" / "wasm32-unknown-unknown" / "release" / "nanpure.wasm"
WASM_DST = ROOT / "web" / "nanpure.wasm"
LOOP_DIR = ROOT / ".loop"

STEPS = [
    ("fmt", ["cargo", "fmt", "--manifest-path", str(MANIFEST), "--check"]),
    (
        "clippy",
        [
            "cargo", "clippy", "--manifest-path", str(MANIFEST),
            "--all-targets", "--", "-D", "warnings",
        ],
    ),
    ("test", ["cargo", "test", "--manifest-path", str(MANIFEST), "--release"]),
    (
        "wasm",
        [
            "cargo", "build", "--manifest-path", str(MANIFEST),
            "--target", "wasm32-unknown-unknown", "--release",
        ],
    ),
]


def main() -> int:
    fast = "--fast" in sys.argv
    LOOP_DIR.mkdir(exist_ok=True)
    ok = True
    for name, cmd in STEPS:
        if fast and name == "wasm":
            print("  [skipped] wasm (--fast)")
            continue
        r = subprocess.run(cmd, capture_output=True, text=True)
        (LOOP_DIR / f"{name}.log").write_text(
            (r.stdout or "") + (r.stderr or ""), encoding="utf-8"
        )
        status = "pass" if r.returncode == 0 else "fail"
        print(f"  [{status}] {name}")
        if r.returncode != 0:
            ok = False
            break
    if ok and not fast:
        shutil.copyfile(WASM_SRC, WASM_DST)
        kb = WASM_DST.stat().st_size // 1024
        print(f"  [pass] deploy web/nanpure.wasm ({kb} KB)")
    print("verify: PASS" if ok else "verify: FAIL — 詳細は .loop/<step>.log")
    return 0 if ok else 1


if __name__ == "__main__":
    sys.exit(main())
