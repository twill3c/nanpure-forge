# AGENTS.md — nanpure-forge

Rust + WASM のナンプレ工房。ロジックは全て Rust(rust/src)・UI は素の HTML/CSS/プレーン JS グルー。
正しさの正本は cargo test の数理ゲート(一意解保証 G-02・mulberry32 言語間一致 G-05)。仕様は SPEC.md、テストは TEST_SPEC.md。

## 1. 技術構成

- ロジック: Rust(`rust/` クレート・edition 2021・外部依存ゼロ)→ `wasm32-unknown-unknown` リリースビルド
- UI: `web/` の index.html + style.css + app.js(プレーン JS グルー)。フレームワーク・ビルドなし
- `web/nanpure.wasm` はビルド成果物としてコミットする(verify.py が再生成・配置)。手で編集しない
- 出荷物に TypeScript / Python を含まない(Python はハーネスと verify.py の開発時のみ)

## 2. looplog 運用の注意

- 新しいイベント種別を初めて使う前に `harness/looplog.py` の EVENT_SPECS(必須フィールドと型)を確認する。推測で引数を組み立てない。
- `test_run` の passed / failed は**直前のテスト出力の数値をそのまま転記**する。記憶で書かない。
- `test_run` の記録はテスト実行と**別コマンド**で行う。実行と記録を同一シェルバッチに
  混ぜると、出力確認前に数値を書くことになる(HC-002)。
- enum フィールド(failure.resolution / severity / commit.kind)の許容値は
  `schema/taxonomy.json` と looplog.py の ENUMS が正。初回使用前に確認する(HC-002)。

## 3. 品質ゲート(完了条件)

`python scripts/verify.py` が green であること。内訳:

| ゲート | 基準 |
|---|---|
| fmt | `cargo fmt --check` 差分 0 |
| clippy | `cargo clippy -- -D warnings` 警告 0 |
| test | `cargo test` 全件 green(数理ゲート G-01〜G-05 含む) |
| wasm | `cargo build --target wasm32-unknown-unknown --release` 成功 + web/ へ配置 |

ゲートを緩める変更(閾値引き下げ、テスト削除・skip、eslint-disable の追加、
ゲート G-xx の基準変更(較正実験の証拠なしの緩和)、`.wt/gate.json` の上限変更)は、人間の承認なしに行わない。

## 4. アーキテクチャ規約

- ナンプレの規則(制約・生成・求解)は **Rust のみ**が知る。app.js は WASM ロード・バッファ入出力・
  DOM 反映だけを行い、盤面の正誤判定ロジックを JS 側に複製しない(N-02)。
- 乱数は mulberry32 を Rust 実装しシード注入(F-01)。`std::time` /乱数クレートを使わない。
- 探索(生成・求解・解数カウント)は必ずノード予算付き(N-03)。無限探索を書かない。
- WASM 境界は 81 バイト共有バッファ + 整数返却のみ(F-04)。文字列・JSON を境界で使わない。
- 縁(空盤・解なし・二解)は正常系として仕様化しテストする。
- 外部クレート依存ゼロを原則とする(標準ライブラリのみ)。

## 5. 変更禁止領域

- `logs/loops/*.jsonl` — append-only(LL-00a)。訂正は correction イベントで。
- AGENTS.md 末尾の scaffold ブロックと `.scaffold/manifest.json` — scaffold-kit 管理。
- `.wt/gate.json` の上限値 — 変更はレジストリ経由(免除パス・test_command の調整は可)。

## 6. よく使うコマンド

```bash
python -m http.server 3000 -d web    # 開発サーバ(静的)
python scripts/verify.py --fast      # fmt + clippy + test(高速ループ用)
python scripts/verify.py             # 上記 + wasm ビルド+配置(完了条件)

python harness/looplog.py append --loop loop_XXX --event ... --data ...
python harness/looplog.py validate
python harness/looplog.py summary --loop loop_XXX

python ../harness-kit/scaffold-kit/scripts/scaffoldctl.py status --registry ../harness-kit/scaffold-kit/registry
```

<!-- scaffold:block agents_core v1.8.0 -->
## 共通規律(scaffold 管理領域 — 手動編集禁止)

このセクションはスキャフォールド・レジストリが管理する。内容を変更したい場合は、
このファイルを直接編集せず、失敗ログ → HARNESS_CHANGELOG 起票 → レジストリ改訂 → `scaffoldctl update` の経路で行うこと。

### 7 段階ループプロトコル

| 段階 | 名称 | 完了条件 |
|---|---|---|
| 1 | 計画 | 対象の要求 ID を特定し、`loop_start` を記録した |
| 2 | 文脈読込 | SPEC.md / IMPLEMENTATION_GUIDE.md の該当箇所と、直近ループのログを読んだ |
| 3 | テスト先行 | TEST_SPEC.md にトレースする失敗するテストを書き、赤を確認した |
| 4 | 実装 | ファイル編集 2 回ごとにテストを実行し、赤のまま次の編集に進んでいない |
| 5 | 検証 | 全テスト合格 + 独立再計算(該当時)を確認した |
| 6 | 文書同期 | SPEC/docs と実装の乖離(SPEC-DRIFT)を解消し、生成ドキュメントを再生成した |
| 7 | 完了 | `loop_end` を記録し、ループログ validate に合格し、専用コミットを積んだ |

### ループ可観測性

全ループは loop-observability の規律(LOOP_LOG_SPEC / FAILURE_TAXONOMY)に従い
`logs/loops/{loop_id}.jsonl` に記録する。失敗は気づいた瞬間に分類コード付きで記録する。
ツーストライク(LL-10)と S1 即時起票(LL-12)は本プロジェクトでも有効である。

### エスカレーション規範

以下の場合は作業を止め、`escalation` を記録してから人間に確認する:
仕様の複数解釈(SPEC-AMB 相当)/ スコープ外ファイルへの変更が必要になった /
破壊的操作(履歴改変・データ削除・強制 push)/ 同種の修正の 3 回目の失敗(PROC-LOOP)。

### コミット規約

Conventional Commits(feat/fix/test/docs/refactor/chore)。スキャフォールド更新は
`chore: scaffold vX.Y.Z` の専用コミットで行い、機能変更と混ぜない。
<!-- /scaffold:block agents_core -->
