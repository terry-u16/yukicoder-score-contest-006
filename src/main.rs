use std::{io::Write, time::Instant};

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
const DEFAULT_SIMULATION_LEN: usize = 40;
const HEIGHT: usize = 60;
const WIDTH: usize = 25;
const CENTER: usize = 12;
const L: usize = !0;
const C: usize = 0;
const R: usize = 1;
const BEAM_WIDTH: [usize; 8] = [20, 15, 10, 5, 5, 5, 3, 3];
const BEAM_CHUNK: usize = 5;

#[derive(Debug, Clone)]
struct State {
    column: usize,
    power: u32,
    raw_score: u32,
    score: f64,
    turn: usize,
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
        }
    }

    fn level(&self) -> u32 {
        self.power / 100
    }

    fn move_player(&mut self, direction: usize) {
        self.column = (self.column + direction + WIDTH) % WIDTH;
    }

    fn attack(&mut self, enemy_collection: &EnemyCollection) {
        let level = self.level();

        if self.enemies.has_enemy(enemy_collection, self.column) {
            let (hp, power) = self.enemies.damage(enemy_collection, self.column, level);
            self.raw_score += hp;
            self.power += power;
        }
    }

    fn clean_up(&mut self, enemy_collection: &EnemyCollection) {
        self.enemies.clean_up_enemies(enemy_collection, self.turn);
    }

    fn progress_turn(&mut self, enemy_collection: &EnemyCollection, direction: usize) -> bool {
        let mut alive = true;
        alive &= !self.enemies.crash(enemy_collection, self.column, self.turn);
        self.move_player(direction);
        alive &= !self.enemies.crash(enemy_collection, self.column, self.turn);
        self.attack(enemy_collection);
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
                let coef = coef * ratio * ratio * 0.5;
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

    fn damage(&mut self, enemies: &EnemyCollection, column: usize, attack: u32) -> (u32, u32) {
        let enemy = enemies.get(column, self.indices[column]).unwrap();
        self.damages[column] += attack;

        if self.damages[column] >= enemy.hp {
            self.damages[column] = 0;
            self.indices[column] += 1;
            (enemy.hp, enemy.power)
        } else {
            (0, 0)
        }
    }

    fn clean_up_enemies(&mut self, enemies: &EnemyCollection, turn: usize) {
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

    fn spawn(&mut self, enemies: &[(u32, u32, usize)], turn: usize) {
        let mut flag = 0;

        for &(hp, power, col) in enemies {
            self.enemies[col].push(Enemy::new(hp, power, turn));
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

fn main() {
    let since = Instant::now();
    let mut state = State::new();
    let mut enemy_collection = EnemyCollection::new();
    let mut turn = 0;

    while let Some(enemies) = read_spawns() {
        enemy_collection.spawn(&enemies, turn);
        let mut all_states = vec![(state.clone(), C)];
        let mut current_states = vec![vec![]; WIDTH];
        current_states[state.column].push(0);
        let simulation_len = DEFAULT_SIMULATION_LEN.min(MAX_TURN - turn);

        for iter in 0..simulation_len {
            let mut next_states = vec![vec![]; WIDTH];
            let beam_width = BEAM_WIDTH[iter / BEAM_CHUNK];

            for &i in current_states.iter().flatten() {
                all_states[i].0.clean_up(&enemy_collection);

                for &dir in &[L, C, R] {
                    let (state, first_dir) = &all_states[i];
                    let mut state = state.clone();
                    let is_alive = state.progress_turn(&enemy_collection, dir);

                    if !is_alive {
                        continue;
                    }

                    let next_col = state.column;
                    let dir = if iter == 0 { dir } else { *first_dir };

                    next_states[next_col].push(all_states.len());
                    all_states.push((state, dir));
                }
            }

            for next in next_states.iter_mut() {
                if next.len() > beam_width {
                    next.select_nth_unstable_by(beam_width, |&i, &j| {
                        all_states[j]
                            .0
                            .score
                            .partial_cmp(&all_states[i].0.score)
                            .unwrap()
                    });
                    next.truncate(beam_width);
                }
            }

            current_states = next_states;
        }

        let mut best_score = std::f64::MIN;
        let mut best_dir = C;

        for (state, dir) in current_states.iter().flatten().map(|&i| &all_states[i]) {
            if best_score.change_max(state.score) {
                best_dir = *dir;
            }
        }

        write_direction(best_dir);
        state.clean_up(&enemy_collection);
        state.progress_turn(&enemy_collection, best_dir);
        turn += 1;

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
