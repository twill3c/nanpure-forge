//! nanpure-forge コア。ナンプレの規則(生成・求解・一意解検証)は全てここが知る(N-02)。
//! 外部クレート依存ゼロ・探索は全てノード予算付き(N-03)。

// ---------------------------------------------------------------- mulberry32

/// フリート共通 PRNG(mulberry32)の Rust 実装(F-01 / G-05)。
/// JS 参照実装とビット単位で一致する(T-001)。
pub struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    pub fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    /// [0, 1) の一様乱数(JS: ((t^(t>>>14))>>>0)/4294967296)
    pub fn next_f64(&mut self) -> f64 {
        self.state = self.state.wrapping_add(0x6d2b_79f5);
        let a = self.state;
        let mut t = (a ^ (a >> 15)).wrapping_mul(1 | a);
        t = t.wrapping_add((t ^ (t >> 7)).wrapping_mul(61 | t)) ^ t;
        f64::from(t ^ (t >> 14)) / 4_294_967_296.0
    }

    /// [0, n) の整数
    pub fn next_below(&mut self, n: usize) -> usize {
        (self.next_f64() * n as f64) as usize
    }
}

// ---------------------------------------------------------------- ソルバー

pub type Grid = [u8; 81];

const ALL: u16 = 0b11_1111_1110; // bit1..bit9

#[inline]
fn box_of(idx: usize) -> usize {
    (idx / 27) * 3 + (idx % 9) / 3
}

struct Masks {
    rows: [u16; 9],
    cols: [u16; 9],
    boxes: [u16; 9],
}

impl Masks {
    /// 盤面からマスクを構築。所与に矛盾があれば None(解なし・T-012)
    fn build(grid: &Grid) -> Option<Self> {
        let mut m = Masks {
            rows: [0; 9],
            cols: [0; 9],
            boxes: [0; 9],
        };
        for (i, &v) in grid.iter().enumerate() {
            if v == 0 {
                continue;
            }
            let bit = 1u16 << v;
            let (r, c, b) = (i / 9, i % 9, box_of(i));
            if m.rows[r] & bit != 0 || m.cols[c] & bit != 0 || m.boxes[b] & bit != 0 {
                return None;
            }
            m.rows[r] |= bit;
            m.cols[c] |= bit;
            m.boxes[b] |= bit;
        }
        Some(m)
    }

    #[inline]
    fn candidates(&self, idx: usize) -> u16 {
        ALL & !(self.rows[idx / 9] | self.cols[idx % 9] | self.boxes[box_of(idx)])
    }

    #[inline]
    fn place(&mut self, idx: usize, v: u8) {
        let bit = 1u16 << v;
        self.rows[idx / 9] |= bit;
        self.cols[idx % 9] |= bit;
        self.boxes[box_of(idx)] |= bit;
    }

    #[inline]
    fn remove(&mut self, idx: usize, v: u8) {
        let bit = !(1u16 << v);
        self.rows[idx / 9] &= bit;
        self.cols[idx % 9] &= bit;
        self.boxes[box_of(idx)] &= bit;
    }
}

/// MRV: 空セルのうち候補最少(タイは最小 index)。埋まっていれば None
fn pick_cell(grid: &Grid, m: &Masks) -> Option<(usize, u16)> {
    let mut best: Option<(usize, u16, u32)> = None;
    for (i, &v) in grid.iter().enumerate() {
        if v != 0 {
            continue;
        }
        let cand = m.candidates(i);
        let n = cand.count_ones();
        match best {
            Some((_, _, bn)) if bn <= n => {}
            _ => best = Some((i, cand, n)),
        }
        if n <= 1 {
            break; // 0(行き詰まり)/ 1(強制)はこれ以上探さない
        }
    }
    best.map(|(i, cand, _)| (i, cand))
}

/// 求解結果(nodes = 消費した探索ノード数・予算検査用)
pub struct SolveOutcome {
    pub solution: Option<Grid>,
    pub nodes: u64,
}

/// digit_order: 候補を試す順(生成器はここをシャッフルして乱択にする)
fn solve_inner(
    grid: &mut Grid,
    m: &mut Masks,
    digit_order: &[u8; 9],
    budget: u64,
    nodes: &mut u64,
) -> bool {
    let Some((idx, cand)) = pick_cell(grid, m) else {
        return true; // 全セル充填
    };
    if cand == 0 {
        return false;
    }
    for &v in digit_order {
        if cand & (1 << v) == 0 {
            continue;
        }
        *nodes += 1;
        if *nodes > budget {
            return false;
        }
        grid[idx] = v;
        m.place(idx, v);
        if solve_inner(grid, m, digit_order, budget, nodes) {
            return true;
        }
        m.remove(idx, v);
        grid[idx] = 0;
    }
    false
}

const ASC: [u8; 9] = [1, 2, 3, 4, 5, 6, 7, 8, 9];

/// バックトラックで求解(F-02)。MRV・候補昇順で決定論。budget 打ち切り
pub fn solve(grid: &Grid, budget: u64) -> SolveOutcome {
    let mut nodes = 0u64;
    let Some(mut m) = Masks::build(grid) else {
        return SolveOutcome {
            solution: None,
            nodes,
        };
    };
    let mut work = *grid;
    let ok = solve_inner(&mut work, &mut m, &ASC, budget, &mut nodes);
    SolveOutcome {
        solution: if ok { Some(work) } else { None },
        nodes,
    }
}

fn count_inner(grid: &mut Grid, m: &mut Masks, budget: u64, nodes: &mut u64, found: &mut u8) {
    if *found >= 2 || *nodes > budget {
        return;
    }
    let Some((idx, cand)) = pick_cell(grid, m) else {
        *found += 1;
        return;
    };
    if cand == 0 {
        return;
    }
    for v in ASC {
        if cand & (1 << v) == 0 {
            continue;
        }
        *nodes += 1;
        if *nodes > budget || *found >= 2 {
            return;
        }
        grid[idx] = v;
        m.place(idx, v);
        count_inner(grid, m, budget, nodes, found);
        m.remove(idx, v);
        grid[idx] = 0;
    }
}

/// 解の個数を上限 2 で数える(F-02)。戻り値 (0/1/2, nodes)
pub fn count_solutions(grid: &Grid, budget: u64) -> (u8, u64) {
    let mut nodes = 0u64;
    let Some(mut m) = Masks::build(grid) else {
        return (0, nodes);
    };
    let mut work = *grid;
    let mut found = 0u8;
    count_inner(&mut work, &mut m, budget, &mut nodes, &mut found);
    (found, nodes)
}

// ---------------------------------------------------------------- 生成器

pub const DIFF_EASY: u32 = 0;
pub const DIFF_NORMAL: u32 = 1;
pub const DIFF_HARD: u32 = 2;

/// 難易度 → 目標ヒント数(F-03)
pub fn target_givens(difficulty: u32) -> usize {
    match difficulty {
        DIFF_EASY => 40,
        DIFF_NORMAL => 32,
        _ => 26,
    }
}

/// 生成結果
pub struct Generated {
    pub puzzle: Grid,
    pub solution: Grid,
    pub givens: usize,
    pub nodes: u64,
}

const GEN_BUDGET: u64 = 40_000_000;
const COUNT_BUDGET: u64 = 400_000;

/// 一意解を保ちながら削る生成器(F-03)。同一シード+難易度で決定論(G-03)
pub fn generate(seed: u32, difficulty: u32) -> Generated {
    let mut rng = Mulberry32::new(seed.wrapping_mul(2654435761).wrapping_add(difficulty));
    let mut nodes = 0u64;

    // 1) 乱択バックトラックで完全盤(候補順をシャッフル)
    let mut order = ASC;
    for i in (1..9).rev() {
        let j = rng.next_below(i + 1);
        order.swap(i, j);
    }
    let mut full: Grid = [0; 81];
    let mut m = Masks::build(&full).expect("空盤は常に妥当");
    let mut n1 = 0u64;
    let ok = solve_inner(&mut full, &mut m, &order, GEN_BUDGET, &mut n1);
    debug_assert!(ok, "空盤の乱択充填は必ず成功する");
    nodes += n1;

    // 2) セルをランダム順に削り、一意性が壊れたら戻す
    let mut cells: [usize; 81] = core::array::from_fn(|i| i);
    for i in (1..81).rev() {
        let j = rng.next_below(i + 1);
        cells.swap(i, j);
    }
    let target = target_givens(difficulty);
    let mut puzzle = full;
    let mut givens = 81usize;
    for &c in cells.iter() {
        if givens <= target {
            break;
        }
        let saved = puzzle[c];
        puzzle[c] = 0;
        let (n, used) = count_solutions(&puzzle, COUNT_BUDGET);
        nodes += used;
        if n == 1 {
            givens -= 1;
        } else {
            puzzle[c] = saved; // 一意性が壊れる(または予算切れで不明)→ 戻す
        }
    }

    Generated {
        puzzle,
        solution: full,
        givens,
        nodes,
    }
}

// ---------------------------------------------------------------- WASM 境界

static mut BUF: [u8; 81] = [0; 81];
static mut SOLUTION: [u8; 81] = [0; 81];

const WASM_SOLVE_BUDGET: u64 = 2_000_000;

/// 共有バッファ(81 バイト・0=空)の先頭ポインタ(F-04)
#[no_mangle]
pub extern "C" fn buf_ptr() -> *mut u8 {
    unsafe { core::ptr::addr_of_mut!(BUF) as *mut u8 }
}

/// 生成してバッファへ書き込み、ヒント数を返す(F-04)
#[no_mangle]
pub extern "C" fn generate_buf(seed: u32, difficulty: u32) -> u32 {
    let g = generate(seed, difficulty);
    unsafe {
        *core::ptr::addr_of_mut!(BUF) = g.puzzle;
        *core::ptr::addr_of_mut!(SOLUTION) = g.solution;
    }
    g.givens as u32
}

/// バッファの盤を解く。成功なら 1(解をバッファへ上書き)・失敗 0(F-04)
#[no_mangle]
pub extern "C" fn solve_buf() -> u32 {
    let grid = unsafe { *core::ptr::addr_of!(BUF) };
    match solve(&grid, WASM_SOLVE_BUDGET).solution {
        Some(sol) => {
            unsafe { *core::ptr::addr_of_mut!(BUF) = sol };
            1
        }
        None => 0,
    }
}

/// バッファの盤の解数(0 / 1 / 2)(F-04)
#[no_mangle]
pub extern "C" fn count_solutions_buf() -> u32 {
    let grid = unsafe { *core::ptr::addr_of!(BUF) };
    u32::from(count_solutions(&grid, WASM_SOLVE_BUDGET).0)
}

/// 直近 generate_buf の正解のセル値(ヒント機能用)
#[no_mangle]
pub extern "C" fn solution_at(idx: u32) -> u32 {
    if idx >= 81 {
        return 0;
    }
    u32::from(unsafe { (*core::ptr::addr_of!(SOLUTION))[idx as usize] })
}

// ================================================================ tests

#[cfg(test)]
mod tests {
    use super::*;

    const SOLVE_BUDGET: u64 = 2_000_000;

    /// テスト内独立検算: 行・列・箱すべてに 1〜9 が一度ずつ
    fn assert_valid_complete(g: &Grid) {
        for unit in 0..9 {
            let mut row = [false; 10];
            let mut col = [false; 10];
            let mut boxx = [false; 10];
            for k in 0..9 {
                let rv = g[unit * 9 + k] as usize;
                let cv = g[k * 9 + unit] as usize;
                let br = (unit / 3) * 3 + k / 3;
                let bc = (unit % 3) * 3 + k % 3;
                let bv = g[br * 9 + bc] as usize;
                assert!((1..=9).contains(&rv), "行に空きか域外");
                assert!(!row[rv], "行重複");
                row[rv] = true;
                assert!(!col[cv], "列重複");
                col[cv] = true;
                assert!(!boxx[bv], "箱重複");
                boxx[bv] = true;
            }
        }
    }

    // T-001 / G-05: mulberry32 の言語間一致(JS 参照実装の実測値)
    #[test]
    fn t001_mulberry32_parity_with_js() {
        let expected_seed1 = [
            0.6270739405881613,
            0.002735721180215478,
            0.5274470399599522,
            0.9810509674716741,
            0.9683778982143849,
            0.281103502959013,
            0.6128388606011868,
            0.7207431411370635,
        ];
        let expected_seed42 = [
            0.6011037519201636,
            0.44829055899754167,
            0.8524657934904099,
            0.6697340414393693,
            0.17481389874592423,
            0.5265925421845168,
            0.2732279943302274,
            0.6247446539346129,
        ];
        for (seed, expected) in [(1u32, expected_seed1), (42u32, expected_seed42)] {
            let mut rng = Mulberry32::new(seed);
            for e in expected {
                let v = rng.next_f64();
                assert_eq!(v, e, "seed {seed} で JS 参照値と不一致");
                assert!((0.0..1.0).contains(&v));
            }
        }
    }

    // T-011: 空盤の求解(決定論・完全盤の妥当性)
    #[test]
    fn t011_solve_empty_board() {
        let empty: Grid = [0; 81];
        let a = solve(&empty, SOLVE_BUDGET);
        let b = solve(&empty, SOLVE_BUDGET);
        let sa = a.solution.expect("空盤は可解");
        assert_valid_complete(&sa);
        assert_eq!(sa, b.solution.unwrap(), "候補順固定なので決定論");
        assert!(a.nodes <= SOLVE_BUDGET);
    }

    // T-010 / G-01: 既知ペア — 完全盤の最終行 9 マスを空けた 72 ヒント盤は
    // 列制約から最終行が一意に定まる(既知の解 = 元の完全盤)
    #[test]
    fn t010_known_pair_last_row_cleared() {
        let full = solve(&[0; 81], SOLVE_BUDGET).solution.unwrap();
        let mut puzzle = full;
        for c in 0..9 {
            puzzle[8 * 9 + c] = 0;
        }
        let (n, _) = count_solutions(&puzzle, SOLVE_BUDGET);
        assert_eq!(n, 1, "72 ヒント(1 行空き)は一意のはず");
        let solved = solve(&puzzle, SOLVE_BUDGET).solution.unwrap();
        assert_eq!(solved, full, "既知の解と完全一致");
        assert_valid_complete(&solved);
    }

    // T-012: 解なし盤 / 二解盤(縁の仕様化)
    #[test]
    fn t012_unsat_and_multiple() {
        // 同じ行に 5 が 2 つ → 解なし
        let mut bad: Grid = [0; 81];
        bad[0] = 5;
        bad[1] = 5;
        assert_eq!(count_solutions(&bad, SOLVE_BUDGET).0, 0);
        assert!(solve(&bad, SOLVE_BUDGET).solution.is_none());
        // 空盤 → 解は 2 つ以上(上限打ち切りで 2)
        assert_eq!(count_solutions(&[0; 81], SOLVE_BUDGET).0, 2);
    }

    // T-020 / G-02 + T-021 / G-03 + T-022 / G-04 + T-100: 生成の横断ゲート
    #[test]
    fn t020_generation_gates() {
        let mut prev_min_givens = usize::MAX;
        for difficulty in [DIFF_EASY, DIFF_NORMAL, DIFF_HARD] {
            let mut min_givens = usize::MAX;
            for seed in 1..=20u32 {
                let g = generate(seed, difficulty);
                // G-03: 決定論
                let g2 = generate(seed, difficulty);
                assert_eq!(g.puzzle, g2.puzzle);
                assert_eq!(g.solution, g2.solution);
                // G-02: 一意解(独立再確認)
                let (n, nodes) = count_solutions(&g.puzzle, SOLVE_BUDGET);
                assert_eq!(n, 1, "seed {seed} diff {difficulty} が一意でない");
                assert!(nodes <= SOLVE_BUDGET);
                // 所与マスは解と一致・解は妥当
                assert_valid_complete(&g.solution);
                let mut givens = 0;
                for i in 0..81 {
                    if g.puzzle[i] != 0 {
                        givens += 1;
                        assert_eq!(g.puzzle[i], g.solution[i]);
                    }
                }
                assert_eq!(givens, g.givens);
                min_givens = min_givens.min(givens);
                // T-100: 生成予算
                assert!(g.nodes <= 50_000_000, "生成予算超過: {}", g.nodes);
            }
            // T-022: ヒント数の難易度域(較正で確定した域)
            match difficulty {
                DIFF_EASY => assert!(min_givens >= 38, "easy min {min_givens}"),
                DIFF_NORMAL => assert!(min_givens <= 36, "normal min {min_givens}"),
                _ => assert!(min_givens <= 30, "hard min {min_givens}"),
            }
            // 難易度間で単調(易しいほどヒントが多い)
            assert!(min_givens <= prev_min_givens);
            prev_min_givens = min_givens;
        }
    }

    // T-030: WASM 境界関数(ネイティブ側で一連の整合を検証)
    #[test]
    fn t030_wasm_boundary_roundtrip() {
        let givens = generate_buf(7, DIFF_NORMAL);
        assert!((17..=81).contains(&givens));
        assert_eq!(count_solutions_buf(), 1);
        // ヒント: 正解セルは 1..=9
        for i in 0..81u32 {
            let v = solution_at(i);
            assert!((1..=9).contains(&v));
        }
        assert_eq!(solve_buf(), 1);
        // 解いた結果が正解と一致
        let ptr = buf_ptr();
        for i in 0..81u32 {
            let v = u32::from(unsafe { *ptr.add(i as usize) });
            assert_eq!(v, solution_at(i));
        }
    }
}
