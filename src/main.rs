use std::{io::Write, time::Instant};

use rand::Xoshiro256;

use crate::{
    beam_width_suggester::{BayesianBeamWidthSuggester, BeamWidthSuggester},
    hash::NopHashSet,
};

macro_rules! get {
      ($t:ty) => {
          {
              let mut line: String = String::new();
              std::io::stdin().read_line(&mut line).unwrap();
              line.trim().parse::<$t>().unwrap()
          }
      };
      ($($t:ty),*) => {
          {
              let mut line: String = String::new();
              std::io::stdin().read_line(&mut line).unwrap();
              let mut iter = line.split_whitespace();
              (
                  $(iter.next().unwrap().parse::<$t>().unwrap(),)*
              )
          }
      };
      ($t:ty; $n:expr) => {
          (0..$n).map(|_|
              get!($t)
          ).collect::<Vec<_>>()
      };
      ($($t:ty),*; $n:expr) => {
          (0..$n).map(|_|
              get!($($t),*)
          ).collect::<Vec<_>>()
      };
      ($t:ty ;;) => {
          {
              let mut line: String = String::new();
              std::io::stdin().read_line(&mut line).unwrap();
              line.split_whitespace()
                  .map(|t| t.parse::<$t>().unwrap())
                  .collect::<Vec<_>>()
          }
      };
      ($t:ty ;; $n:expr) => {
          (0..$n).map(|_| get!($t ;;)).collect::<Vec<_>>()
      };
}

pub trait ChangeMinMax {
    fn change_min(&mut self, v: Self) -> bool;
    fn change_max(&mut self, v: Self) -> bool;
}

impl<T: PartialOrd> ChangeMinMax for T {
    fn change_min(&mut self, v: T) -> bool {
        *self > v && {
            *self = v;
            true
        }
    }

    fn change_max(&mut self, v: T) -> bool {
        *self < v && {
            *self = v;
            true
        }
    }
}

const MAX_TURN: usize = 1000;
const DEFAULT_SIMULATION_LEN: usize = 30;
const HEIGHT: usize = 60;
const WIDTH: usize = 25;
const CENTER: usize = 12;
const L: usize = !0;
const C: usize = 0;
const R: usize = 1;
const BEAM_WIDTH: usize = 15;
const TURN_STRIDE: usize = 2;

#[derive(Debug, Clone)]
struct State {
    column: usize,
    power: u32,
    raw_score: u32,
    score: f64,
    turn: usize,
    hash: u64,
    enemies: EnemyState,
}

impl State {
    fn new() -> Self {
        Self {
            column: CENTER,
            power: 100,
            raw_score: 0,
            turn: 0,
            score: 0.0,
            enemies: EnemyState::new(),
            hash: 0,
        }
    }

    fn level(&self) -> u32 {
        self.power / 100
    }

    fn move_player(&mut self, direction: usize) {
        self.column = (self.column + direction + WIDTH) % WIDTH;
    }

    fn attack(&mut self, enemy_collection: &EnemyCollection, hash: &ZobristHash) {
        let level = self.level();

        if self.enemies.has_enemy(enemy_collection, self.column) {
            let (hp, power) =
                self.enemies
                    .damage(enemy_collection, self.column, level, hash, &mut self.hash);
            self.raw_score += hp;
            self.power += power;
        }
    }

    fn clean_up(&mut self, enemy_collection: &EnemyCollection, hash: &ZobristHash) {
        self.enemies
            .clean_up_enemies(enemy_collection, self.turn, hash, &mut self.hash);
    }

    fn progress_turn(
        &mut self,
        enemy_collection: &EnemyCollection,
        hash: &ZobristHash,
        direction: usize,
    ) -> bool {
        let mut alive = true;
        alive &= !self.enemies.crash(enemy_collection, self.column, self.turn);
        self.move_player(direction);
        alive &= !self.enemies.crash(enemy_collection, self.column, self.turn);
        self.attack(enemy_collection, hash);
        self.turn += 1;

        self.update_score(enemy_collection);

        alive
    }

    fn update_score(&mut self, enemy_collection: &EnemyCollection) {
        let mut raw_score_point = self.raw_score as f64;
        let mut power_point = self.power as f64;
        let cols = [
            ((self.column + WIDTH - L) % WIDTH, 0.5),
            (self.column, 1.0),
            ((self.column + R) % WIDTH, 0.5),
        ];

        for &(col, coef) in &cols {
            if let Some(enemy) = self.enemies.get(enemy_collection, col) {
                let ratio = self.enemies.damages[col] as f64 / enemy.hp as f64;

                if ratio == 0.0 {
                    continue;
                }

                let coef = coef * ratio * ratio;
                raw_score_point += enemy.hp as f64 * coef;
                power_point += enemy.power as f64 * coef;
            }
        }

        let raw_score_coef = (self.turn * self.turn) as f64;
        let power_point_coef = ((MAX_TURN - self.turn) * MAX_TURN) as f64;
        self.score = raw_score_point * raw_score_coef + power_point * power_point_coef;
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Enemy {
    hp: u32,
    power: u32,
    spawn_turn: usize,
}

impl Enemy {
    fn new(hp: u32, power: u32, spawn_turn: usize) -> Self {
        Self {
            hp,
            power,
            spawn_turn,
        }
    }

    fn is_out_of_range(&self, turn: usize) -> bool {
        self.spawn_turn + HEIGHT <= turn
    }

    fn is_bottom(&self, turn: usize) -> bool {
        self.spawn_turn + HEIGHT - 1 == turn
    }
}

#[derive(Debug, Clone)]
struct EnemyState {
    indices: [usize; WIDTH],
    damages: [u32; WIDTH],
}

impl EnemyState {
    fn new() -> Self {
        Self {
            indices: [0; WIDTH],
            damages: [0; WIDTH],
        }
    }

    fn has_enemy(&self, enemies: &EnemyCollection, column: usize) -> bool {
        self.get(enemies, column).is_some()
    }

    fn get<'a>(&self, enemies: &'a EnemyCollection, column: usize) -> Option<&'a Enemy> {
        enemies.get(column, self.indices[column])
    }

    fn crash(&self, enemies: &EnemyCollection, column: usize, turn: usize) -> bool {
        if let Some(enemy) = enemies.get(column, self.indices[column]) {
            enemy.is_bottom(turn)
        } else {
            false
        }
    }

    fn damage(
        &mut self,
        enemies: &EnemyCollection,
        column: usize,
        attack: u32,
        hashes: &ZobristHash,
        hash: &mut u64,
    ) -> (u32, u32) {
        let enemy = enemies.get(column, self.indices[column]).unwrap();
        let damage = &mut self.damages[column];
        *hash ^= hashes.get(enemy.spawn_turn, column, enemy.hp - *damage);
        *damage += attack;
        *hash ^= hashes.get(enemy.spawn_turn, column, enemy.hp.saturating_sub(*damage));

        if self.damages[column] >= enemy.hp {
            self.damages[column] = 0;
            self.indices[column] += 1;
            (enemy.hp, enemy.power)
        } else {
            (0, 0)
        }
    }

    fn clean_up_enemies(
        &mut self,
        enemies: &EnemyCollection,
        turn: usize,
        hashes: &ZobristHash,
        hash: &mut u64,
    ) {
        let mut column = 0;
        let mut flag = enemies.clean_flags[turn];

        while flag > 0 {
            let tzcnt = flag.trailing_zeros();
            flag >>= tzcnt;
            column += tzcnt;

            let index = &mut self.indices[column as usize];
            let damage = &mut self.damages[column as usize];

            if let Some(enemy) = enemies.get(column as usize, *index) {
                if enemy.is_out_of_range(turn) {
                    *hash ^= hashes.get(enemy.spawn_turn, column as usize, enemy.hp - *damage);
                    *damage = 0;
                    *index += 1;
                }
            }

            flag >>= 1;
            column += 1;
        }
    }
}

#[derive(Debug, Clone)]
struct EnemyCollection {
    enemies: Vec<Vec<Enemy>>,
    clean_flags: Vec<u32>,
}

impl EnemyCollection {
    fn new() -> Self {
        Self {
            enemies: vec![vec![]; WIDTH],
            clean_flags: vec![0; MAX_TURN],
        }
    }

    fn spawn(
        &mut self,
        enemies: &[(u32, u32, usize)],
        hashes: &ZobristHash,
        hash: &mut u64,
        turn: usize,
    ) {
        let mut flag = 0;

        for &(hp, power, col) in enemies {
            self.enemies[col].push(Enemy::new(hp, power, turn));
            *hash ^= hashes.get(turn, col, hp);
            flag |= 1 << col;
        }

        if turn + HEIGHT < MAX_TURN {
            self.clean_flags[turn + HEIGHT] = flag;
        }
    }

    fn get(&self, column: usize, index: usize) -> Option<&Enemy> {
        self.enemies[column].get(index)
    }
}

struct ZobristHash {
    hashes: Vec<u64>,
}

impl ZobristHash {
    const MAX_HP: usize = 500;

    fn new() -> Self {
        let mut hashes = vec![0; MAX_TURN * WIDTH * Self::MAX_HP];
        let mut rng = Xoshiro256::new(42);

        let mut index = 0;

        for _ in 0..MAX_TURN {
            for _ in 0..WIDTH {
                // HP0のときはhashも0とする
                index += 1;

                for _ in 1..Self::MAX_HP {
                    hashes[index] = rng.next();
                    index += 1;
                }
            }
        }

        Self { hashes }
    }

    fn get(&self, turn: usize, col: usize, hp: u32) -> u64 {
        self.hashes[(turn * WIDTH + col) * Self::MAX_HP + hp as usize]
    }
}

fn main() {
    let since = Instant::now();
    let mut state = State::new();
    let mut enemy_collection = EnemyCollection::new();
    let mut turn = 0;
    let mut width_suggester = BayesianBeamWidthSuggester::new(
        MAX_TURN / TURN_STRIDE,
        20 / TURN_STRIDE,
        1.98,
        BEAM_WIDTH,
        1,
        BEAM_WIDTH * 10,
        50,
    );

    let hash = ZobristHash::new();

    // 助けてくれ
    let mut hashset: [NopHashSet<u64>; WIDTH] = [
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
        NopHashSet::default(),
    ];

    let mut all_states = vec![];
    let mut current_states = vec![vec![]; WIDTH];

    while let Some(enemies) = read_spawns() {
        let beam_width = width_suggester.suggest();
        enemy_collection.spawn(&enemies, &hash, &mut state.hash, turn);
        all_states.clear();
        all_states.push((state.clone(), [C; TURN_STRIDE]));

        for s in current_states.iter_mut() {
            s.clear();
        }

        current_states[state.column].push(0);
        let simulation_len = DEFAULT_SIMULATION_LEN.min(MAX_TURN - turn);

        for iter in 0..simulation_len {
            let mut next_states = vec![Vec::with_capacity(beam_width * 3); WIDTH];

            for s in hashset.iter_mut() {
                s.clear();
            }

            for &i in current_states.iter().flatten() {
                all_states[i].0.clean_up(&enemy_collection, &hash);

                for &dir in &[L, C, R] {
                    let (state, directions) = &all_states[i];
                    let mut state = state.clone();
                    let is_alive = state.progress_turn(&enemy_collection, &hash, dir);

                    if !is_alive {
                        continue;
                    }

                    let next_col = state.column;
                    let mut directions = directions.clone();

                    if iter < TURN_STRIDE {
                        directions[iter] = dir;
                    }

                    next_states[next_col].push(all_states.len());
                    all_states.push((state, directions));
                }
            }

            for (next, hashset) in next_states.iter_mut().zip(hashset.iter_mut()) {
                next.sort_unstable_by(|&i, &j| {
                    all_states[j]
                        .0
                        .score
                        .partial_cmp(&all_states[i].0.score)
                        .unwrap()
                });

                next.retain(|&i| hashset.insert(all_states[i].0.hash));
                next.truncate(beam_width);
            }

            current_states = next_states;
        }

        let mut best_score = std::f64::MIN;
        let mut best_dir = [C; TURN_STRIDE];

        for (state, dir) in current_states.iter().flatten().map(|&i| &all_states[i]) {
            if best_score.change_max(state.score) {
                best_dir = dir.clone();
            }
        }

        write_direction(best_dir[0]);
        state.clean_up(&enemy_collection, &hash);
        state.progress_turn(&enemy_collection, &hash, best_dir[0]);
        turn += 1;

        for i in 1..TURN_STRIDE {
            if let Some(enemies) = read_spawns() {
                enemy_collection.spawn(&enemies, &hash, &mut state.hash, turn);
                write_direction(best_dir[i]);
                state.clean_up(&enemy_collection, &hash);
                state.progress_turn(&enemy_collection, &hash, best_dir[i]);
                turn += 1;
            }
        }

        if turn == MAX_TURN {
            break;
        }
    }

    eprintln!("final score: {}", state.raw_score);
    eprintln!("{:.3}s", (Instant::now() - since).as_secs_f64());
}

fn read_spawns() -> Option<Vec<(u32, u32, usize)>> {
    let n = get!(i32);

    if n < 0 {
        return None;
    }

    let mut enemies = vec![];

    for _ in 0..n {
        enemies.push(get!(u32, u32, usize));
    }

    Some(enemies)
}

fn write_direction(direction: usize) {
    match direction {
        L => println!("L"),
        C => println!("S"),
        R => println!("R"),
        _ => unreachable!(),
    }

    std::io::stdout().flush().unwrap();
}

mod beam_width_suggester {
    use std::time::Instant;

    /// ビーム幅を提案するトレイト
    pub trait BeamWidthSuggester {
        // 現在のターン数を受け取り、ビーム幅を提案する
        fn suggest(&mut self) -> usize;
    }

    /// ベイズ推定+カルマンフィルタにより適切なビーム幅を計算するBeamWidthSuggester。
    /// 1ターンあたりの実行時間が正規分布に従うと仮定し、+3σ分の余裕を持ってビーム幅を決める。
    ///
    /// ## モデル
    ///
    /// カルマンフィルタを適用するにあたって、以下のモデルを考える。
    ///
    /// - `i` ターン目のビーム幅1あたりの所要時間の平均値 `t_i` が正規分布 `N(μ_i, σ_i^2)` に従うと仮定する。
    ///   - 各ターンに観測される所要時間が `N(μ_i, σ_i^2)` に従うのではなく、所要時間の**平均値**が `N(μ_i, σ_i^2)` に従うとしている点に注意。
    ///     - すなわち `μ_i` は所要時間の平均値の平均値であり、所要時間の平均値が `μ_i` を中心とした確率分布を形成しているものとしている。ややこしい。
    ///   - この `μ_i` , `σ_i^2` をベイズ推定によって求めたい。
    /// - 所要時間 `t_i` は `t_{i+1}=t_i+N(0, α^2)` により更新されるものとする。
    ///   - `N(0, α^2)` は標準偏差 `α` のノイズを意味する。お気持ちとしては「実行時間がターン経過に伴ってちょっとずつ変わっていくことがあるよ」という感じ。
    ///   - `α` は既知の定数とし、適当に決める。
    ///   - 本来は問題に合わせたちゃんとした更新式にすべき（ターン経過に伴って線形に増加するなど）なのだが、事前情報がないため大胆に仮定する。
    /// - 所要時間の観測値 `τ_i` は `τ_i=t_i+N(0, β^2)` により得られるものとする。
    ///   - `β` は既知の定数とし、適当に決める。
    ///   - 本来この `β` も推定できると嬉しいのだが、取扱いが煩雑になるためこちらも大胆に仮定する。
    ///
    /// ## モデルの初期化
    ///
    /// - `μ_0` は実行時間制限を `T` 、標準ビーム幅を `W` 、実行ターン数を `M` として、 `μ_0=T/WM` などとすればよい。
    /// - `σ_0` は適当に `σ_0=0.1μ_0` とする。ここは標準ビーム幅にどのくらい自信があるかによる。
    /// - `α` は適当に `α=0.01μ_0` とする。定数は本当に勘。多分問題に合わせてちゃんと考えた方が良い。
    /// - `β` は `σ_0=0.05μ_0` とする。適当なベンチマーク問題で標準偏差を取ったらそのくらいだったため。
    ///
    /// ## モデルの更新
    ///
    /// 以下のように更新をかけていく。
    ///
    /// 1. `t_0=N(μ_0, σ_0^2)` と初期化する。
    /// 2. `t_1=t_0+N(0, α^2)` とし、事前分布 `t_1=N(μ_1, σ_1^2)=N(μ_0, σ_0^2+α^2)` を得る。ここはベイズ更新ではなく単純な正規分布の合成でよい。
    /// 3. `τ_1` が観測されるので、ベイズ更新して事後分布 `N(μ_1', σ_1^2')` を得る。
    /// 4. 同様に `t_2=N(μ_2, σ_2^2)` を得る。
    /// 5. `τ_2` を用いてベイズ更新。以下同様。
    ///
    /// ## 適切なビーム幅の推定
    ///
    /// - 余裕を持って、99.8%程度の確率（+3σ）で実行時間制限に収まるようなビーム幅にしたい。
    /// - ここで、 `t_i=t_{i+1}=･･･=t_M=N(μ_i, σ_i^2)` と大胆仮定する。
    ///   - `α` によって `t_i` がどんどん変わってしまうと考えるのは保守的すぎるため。
    /// - すると残りターン数 `M_i=M-i` として、 `Στ_i=N(M_i*μ_i, M_i*σ_i^2)` となる。
    /// - したがって、残り時間を `T_i` として `W(M_i*μ_i+3(σ_i√M_i))≦T_i` となる最大の `W` を求めればよく、 `W=floor(T_i/(M_i*μ_i+3(σ_i√M_i)))` となる。
    /// - 最後に、念のため適当な `W_min` , `W_max` でclampしておく。
    pub struct BayesianBeamWidthSuggester {
        /// ビーム幅1あたりの所要時間の平均値の平均値μ_i（逐次更新される）
        mean_sec: f64,
        /// ビーム幅1あたりの所要時間の平均値の分散σ_i^2（逐次更新される）
        variance_sec: f64,
        /// 1ターンごとに状態に作用するノイズの大きさを表す分散α^2（定数）
        variance_state_sec: f64,
        /// 観測時に乗るノイズの大きさを表す分散β^2（定数）
        variance_observe_sec: f64,
        /// 問題の実行時間制限T
        time_limit_sec: f64,
        /// 現在のターン数i
        current_turn: usize,
        /// 最大ターン数M
        max_turn: usize,
        /// ウォームアップターン数（最初のXターン分の情報は採用せずに捨てる）
        warmup_turn: usize,
        /// 最小ビーム幅W_min
        min_beam_width: usize,
        /// 最大ビーム幅W_max
        max_beam_width: usize,
        /// 現在のビーム幅W_i
        current_beam_width: usize,
        /// ログの出力インターバル（0にするとログを出力しなくなる）
        verbose_interval: usize,
        /// ビーム開始時刻
        start_time: Instant,
        /// 前回の計測時刻
        last_time: Instant,
    }

    impl BayesianBeamWidthSuggester {
        pub fn new(
            max_turn: usize,
            warmup_turn: usize,
            time_limit_sec: f64,
            standard_beam_width: usize,
            min_beam_width: usize,
            max_beam_width: usize,
            verbose_interval: usize,
        ) -> Self {
            assert!(
                max_turn * standard_beam_width > 0,
                "ターン数とビーム幅設定が不正です。"
            );
            assert!(
                min_beam_width > 0,
                "最小のビーム幅は正の値でなければなりません。"
            );
            assert!(
                min_beam_width <= max_beam_width,
                "最大のビーム幅は最小のビーム幅以上でなければなりません。"
            );

            let mean_sec = time_limit_sec / (max_turn * standard_beam_width) as f64;

            // 雑にσ=10%ズレると仮定
            let stddev_sec = 0.1 * mean_sec;
            let variance_sec = stddev_sec * stddev_sec;
            let stddev_state_sec = 0.01 * mean_sec;
            let variance_state_sec = stddev_state_sec * stddev_state_sec;
            let stddev_observe_sec = 0.05 * mean_sec;
            let variance_observe_sec = stddev_observe_sec * stddev_observe_sec;

            eprintln!(
                "standard beam width: {}, time limit: {:.3}s",
                standard_beam_width, time_limit_sec
            );

            Self {
                mean_sec,
                variance_sec,
                time_limit_sec,
                variance_state_sec,
                variance_observe_sec,
                current_turn: 0,
                min_beam_width,
                max_beam_width,
                verbose_interval,
                max_turn,
                warmup_turn,
                current_beam_width: 0,
                start_time: Instant::now(),
                last_time: Instant::now(),
            }
        }

        fn update_state(&mut self) {
            // N(0, α^2)のノイズが乗る
            self.variance_sec += self.variance_state_sec;
        }

        fn update_distribution(&mut self, duration_sec: f64) {
            let old_mean = self.mean_sec;
            let old_variance = self.variance_sec;
            let noise_variance = self.variance_observe_sec;

            self.mean_sec = (old_mean * noise_variance + old_variance * duration_sec)
                / (noise_variance + old_variance);
            self.variance_sec = old_variance * noise_variance / (old_variance + noise_variance);
        }

        fn calc_safe_beam_width(&self) -> usize {
            let remaining_turn = (self.max_turn - self.current_turn) as f64;
            let elapsed_time = (Instant::now() - self.start_time).as_secs_f64();
            let remaining_time = self.time_limit_sec - elapsed_time;

            // 平均値の分散σ^2と観測ノイズβ^2が乗ってくると考える
            let variance_total = self.variance_sec + self.variance_observe_sec;

            // N(ξ, η^2)からのサンプリングをK回繰り返すとN(Kξ, Kη^2)となる（はず）
            let mean = remaining_turn * self.mean_sec;
            let variance = remaining_turn * variance_total;
            let stddev = variance.sqrt();

            // 3σの余裕を持たせる
            const SIGMA_COEF: f64 = 3.0;
            let needed_time_per_width = mean + SIGMA_COEF * stddev;
            let beam_width = ((remaining_time / needed_time_per_width) as usize)
                .max(self.min_beam_width)
                .min(self.max_beam_width);

            if self.verbose_interval != 0 && self.current_turn % self.verbose_interval == 0 {
                let stddev_per_run = (self.max_turn as f64 * variance_total).sqrt();
                let stddev_per_turn = variance_total.sqrt();

                eprintln!(
                    "turn: {:4}, beam width: {:4}, pase: {:.3}±{:.3}ms/run, iter time: {:.3}±{:.3}ms",
                    self.current_turn,
                    beam_width,
                    self.mean_sec * (beam_width * self.max_turn) as f64 * 1e3,
                    stddev_per_run * beam_width as f64 * 1e3,
                    self.mean_sec * beam_width as f64 * 1e3,
                    stddev_per_turn * beam_width as f64 * 1e3
                );
            }

            beam_width
        }
    }

    impl BeamWidthSuggester for BayesianBeamWidthSuggester {
        fn suggest(&mut self) -> usize {
            assert!(
                self.current_turn < self.max_turn,
                "規定ターン終了後にsuggest()が呼び出されました。"
            );

            if self.current_turn >= self.warmup_turn {
                let elapsed = (Instant::now() - self.last_time).as_secs_f64();
                let elapsed_per_beam = elapsed / self.current_beam_width as f64;
                self.update_state();
                self.update_distribution(elapsed_per_beam);
            }

            self.last_time = Instant::now();
            let beam_width = self.calc_safe_beam_width();
            self.current_beam_width = beam_width;
            self.current_turn += 1;
            beam_width
        }
    }
}

#[allow(dead_code)]
mod rand {
    pub(crate) struct Xoshiro256 {
        s0: u64,
        s1: u64,
        s2: u64,
        s3: u64,
    }

    impl Xoshiro256 {
        pub(crate) fn new(mut seed: u64) -> Self {
            let s0 = split_mix_64(&mut seed);
            let s1 = split_mix_64(&mut seed);
            let s2 = split_mix_64(&mut seed);
            let s3 = split_mix_64(&mut seed);
            Self { s0, s1, s2, s3 }
        }

        pub fn next(&mut self) -> u64 {
            let result = (self.s1 * 5).rotate_left(7) * 9;
            let t = self.s1 << 17;

            self.s2 ^= self.s0;
            self.s3 ^= self.s1;
            self.s1 ^= self.s2;
            self.s0 ^= self.s3;
            self.s2 ^= t;
            self.s3 = self.s3.rotate_left(45);

            result
        }

        pub(crate) fn gen_usize(&mut self, lower: usize, upper: usize) -> usize {
            assert!(lower < upper);
            let count = upper - lower;
            (self.next() % count as u64) as usize + lower
        }

        pub(crate) fn gen_i32(&mut self, lower: i32, upper: i32) -> i32 {
            assert!(lower < upper);
            let count = upper - lower;
            (self.next() % count as u64) as i32 + lower
        }

        pub(crate) fn gen_f64(&mut self) -> f64 {
            const UPPER_MASK: u64 = 0x3ff0000000000000;
            const LOWER_MASK: u64 = 0xfffffffffffff;
            let result = UPPER_MASK | (self.next() & LOWER_MASK);
            let result: f64 = unsafe { std::mem::transmute(result) };
            result - 1.0
        }

        pub(crate) fn gen_bool(&mut self, prob: f64) -> bool {
            self.gen_f64() < prob
        }
    }

    fn split_mix_64(x: &mut u64) -> u64 {
        *x += 0x9e3779b97f4a7c15;
        let mut z = *x;
        z = (z ^ z >> 30) * 0xbf58476d1ce4e5b9;
        z = (z ^ z >> 27) * 0x94d049bb133111eb;
        return z ^ z >> 31;
    }
}

#[allow(dead_code)]
mod hash {
    use core::hash::BuildHasherDefault;
    use core::hash::Hasher;
    use std::collections::{HashMap, HashSet};

    #[derive(Default)]
    pub struct NopHasher {
        hash: u64,
    }
    impl Hasher for NopHasher {
        fn write(&mut self, _: &[u8]) {
            panic!();
        }

        #[inline]
        fn write_u64(&mut self, n: u64) {
            self.hash = n;
        }

        #[inline]
        fn finish(&self) -> u64 {
            self.hash
        }
    }

    pub type NopHashMap<K, V> = HashMap<K, V, BuildHasherDefault<NopHasher>>;
    pub type NopHashSet<V> = HashSet<V, BuildHasherDefault<NopHasher>>;
}
