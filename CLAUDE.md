# CLAUDE.md

@AGENTS.md

上記ハーネスがこのリポジトリの正本ルール。要点のみ再掲する:

- 仕様の正本は SPEC.md。変更は スペック → テスト → 実装 の順。
- すべてのタスクは 7 段階ループプロトコル(AGENTS.md 末尾の共通規律)で進め、
  `python harness/looplog.py append` で `logs/loops/{loop_id}.jsonl` に記録する。
  失敗は気づいた瞬間に FAILURE_TAXONOMY のコード付きで記録する。
- 完了条件は `python scripts/verify.py` green + `looplog.py validate` 合格。
- ロジックは全て Rust(rust/src)。app.js はグルーのみでナンプレ規則を知らない(N-02)。
  決定性(同一シード+難易度 → 同一盤面)を壊さない。
- 正しさの正本は cargo test の数理ゲート(一意解保証 G-02・mulberry32 言語間一致 G-05)。web/nanpure.wasm は verify.py だけが生成する。学習ゲート G-xx の数値は
  較正実験の証拠付きでのみ変更できる(緩和は人間の承認が必要)。
- scaffold ブロック(AGENTS.md 末尾)と `.wt/gate.json` の上限は直接編集しない。
