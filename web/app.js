// nanpure-forge — プレーン JS グルー。
// ナンプレの規則はここには存在しない(N-02)。重複検査・完成判定・生成・求解は
// 全て WASM(Rust)側の関数を呼ぶだけで、JS は入出力と DOM 反映のみを担う。
"use strict";

const DIFF_LABELS = ["やさしい", "ふつう", "むずかしい"];

let wasm = null; // WASM exports
let bufPtr = 0; // 81 バイト共有バッファの先頭
let given = new Array(81).fill(false); // 所与マス(操作不可)
let selected = -1; // 選択中セル index(-1 = なし)
let difficulty = 1;
let seed = 1;
let done = false;
let startedAt = 0;
let timerId = 0;

const $ = (id) => document.getElementById(id);
const cellEls = [];

/** 共有バッファのビュー(memory 成長はないが毎回取り直すのが安全) */
function cells() {
  return new Uint8Array(wasm.memory.buffer, bufPtr, 81);
}

// ---------------------------------------------------------------- DOM 構築

function buildBoard() {
  const board = $("board");
  for (let i = 0; i < 81; i++) {
    const b = document.createElement("button");
    b.type = "button";
    b.className = "cell";
    const col = i % 9;
    const row = (i / 9) | 0;
    if (col === 2 || col === 5) b.classList.add("br3");
    if (row === 2 || row === 5) b.classList.add("bb3");
    b.dataset.idx = String(i);
    b.addEventListener("click", () => selectCell(i));
    board.appendChild(b);
    cellEls.push(b);
  }
}

function buildPad() {
  const pad = $("pad");
  for (let d = 1; d <= 9; d++) {
    const b = document.createElement("button");
    b.type = "button";
    b.textContent = String(d);
    b.addEventListener("click", () => inputDigit(d));
    pad.appendChild(b);
  }
  const del = document.createElement("button");
  del.type = "button";
  del.textContent = "✕";
  del.setAttribute("aria-label", "消す");
  del.addEventListener("click", () => inputDigit(0));
  pad.appendChild(del);
}

// ---------------------------------------------------------------- 描画

function render() {
  const g = cells();
  for (let i = 0; i < 81; i++) {
    const el = cellEls[i];
    el.textContent = g[i] === 0 ? "" : String(g[i]);
    el.classList.toggle("given", given[i]);
    el.classList.toggle("selected", i === selected);
    // 重複検査は Rust に訊く(N-02)
    el.classList.toggle("conflict", wasm.conflict_at(i) === 1);
  }
}

function fmtTime(ms) {
  const s = Math.floor(ms / 1000);
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, "0")}`;
}

function tick() {
  $("time").textContent = fmtTime(Date.now() - startedAt);
}

// ---------------------------------------------------------------- 操作

function selectCell(i) {
  if (done || given[i]) return;
  selected = selected === i ? -1 : i;
  render();
}

function inputDigit(d) {
  if (done || selected < 0 || given[selected]) return;
  cells()[selected] = d;
  render();
  checkComplete();
}

function checkComplete() {
  // 完成判定も Rust に訊く(N-02)
  if (wasm.is_complete_valid_buf() !== 1) return;
  done = true;
  selected = -1;
  clearInterval(timerId);
  $("banner-time").textContent = `タイム ${fmtTime(Date.now() - startedAt)}`;
  $("banner").hidden = false;
  render();
}

function newPuzzle(diff) {
  difficulty = diff;
  seed = (seed + 1) >>> 0;
  const givens = wasm.generate_buf(seed, difficulty);
  const g = cells();
  for (let i = 0; i < 81; i++) given[i] = g[i] !== 0;
  selected = -1;
  done = false;
  $("banner").hidden = true;
  $("diff-label").textContent = DIFF_LABELS[difficulty];
  $("givens").textContent = String(givens);
  $("seed").textContent = `#${seed}`;
  startedAt = Date.now();
  clearInterval(timerId);
  timerId = setInterval(tick, 1000);
  tick();
  render();
  document.querySelectorAll("[data-diff]").forEach((b) => {
    b.classList.toggle("active", Number(b.dataset.diff) === difficulty);
  });
}

function checkAnswers() {
  if (done) return;
  const g = cells();
  for (let i = 0; i < 81; i++) {
    if (given[i] || g[i] === 0) continue;
    // 正解値は Rust が保持する解答盤から(N-02)
    if (g[i] !== wasm.solution_at(i)) {
      const el = cellEls[i];
      el.classList.remove("wrong");
      void el.offsetWidth; // アニメーション再発火
      el.classList.add("wrong");
    }
  }
}

function hint() {
  if (done) return;
  const g = cells();
  let target = selected >= 0 && !given[selected] && g[selected] === 0 ? selected : -1;
  if (target < 0) {
    for (let i = 0; i < 81; i++) {
      if (!given[i] && g[i] === 0) {
        target = i;
        break;
      }
    }
  }
  if (target < 0) return;
  g[target] = wasm.solution_at(target);
  selected = target;
  render();
  checkComplete();
}

function clearAll() {
  if (done) return;
  const g = cells();
  for (let i = 0; i < 81; i++) {
    if (!given[i]) g[i] = 0;
  }
  selected = -1;
  render();
}

// ---------------------------------------------------------------- 起動

function wire() {
  document.querySelectorAll("[data-diff]").forEach((b) => {
    b.addEventListener("click", () => newPuzzle(Number(b.dataset.diff)));
  });
  $("check").addEventListener("click", checkAnswers);
  $("hint").addEventListener("click", hint);
  $("clear").addEventListener("click", clearAll);
  document.addEventListener("keydown", (e) => {
    if (e.key >= "1" && e.key <= "9") inputDigit(Number(e.key));
    else if (e.key === "Backspace" || e.key === "Delete" || e.key === "0") inputDigit(0);
  });
}

async function init() {
  const res = await WebAssembly.instantiateStreaming(fetch("nanpure.wasm"));
  wasm = res.instance.exports;
  bufPtr = wasm.buf_ptr();
  buildBoard();
  buildPad();
  wire();
  // 初回の問題番号は日付ベース(UI 層の趣向。コアはシードに対して決定論)
  seed = (Date.now() % 900000) >>> 0;
  newPuzzle(1);
}

init();
