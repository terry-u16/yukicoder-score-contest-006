use std::{collections::VecDeque, io::Write};

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
const DEFAULT_SIMULATION_LEN: usize = 15;
const HEIGHT: usize = 60;
const WIDTH: usize = 25;
const CENTER: usize = 12;
const L: usize = !0;
const C: usize = 0;
const R: usize = 1;

#[derive(Debug, Clone, Default)]
struct State {
    column: usize,
    power: u32,
    raw_score: u32,
    board: Vec<u64>,
    turn: usize,
    enemies: Vec<VecDeque<Enemy>>,
}

impl State {
    fn new() -> Self {
        Self {
            column: CENTER,
            power: 100,
            raw_score: 0,
            board: vec![0; WIDTH],
            turn: 0,
            enemies: vec![VecDeque::new(); WIDTH],
        }
    }

    fn level(&self) -> u32 {
        self.power / 100
    }

    fn move_enemy(&mut self) -> bool {
        for (i, b) in self.board.iter_mut().enumerate() {
            if *b & 1 > 0 {
                self.enemies[i].pop_front();
            }

            *b >>= 1;
        }

        !self.is_crash()
    }

    fn spawn(&mut self, enemies: &[(u32, u32, usize)]) {
        for &(hp, power, col) in enemies {
            self.board[col] |= 1 << (HEIGHT - 1);
            self.enemies[col].push_back(Enemy::new(hp, power));
        }
    }

    fn move_player(&mut self, direction: usize) -> bool {
        self.column = (self.column + direction + WIDTH) % WIDTH;
        !self.is_crash()
    }

    fn attack(&mut self) {
        let level = self.level();

        if let Some(enemy) = self.enemies[self.column].front_mut() {
            enemy.damage(level);

            if enemy.hp == 0 {
                self.raw_score += enemy.init_hp;
                self.power += enemy.power;
                self.enemies[self.column].pop_front();

                // ビットを下ろす
                let signed = self.board[self.column] as i64;
                let lsb = signed & -signed;
                self.board[self.column] ^= lsb as u64;
            }
        }
    }

    fn progress_turn(&mut self, enemies: &[(u32, u32, usize)], direction: usize) -> bool {
        let mut alive = true;
        alive &= self.move_enemy();
        self.spawn(enemies);
        alive &= self.move_player(direction);
        self.attack();
        self.turn += 1;

        alive
    }

    fn is_crash(&self) -> bool {
        (self.board[self.column] & 1) > 0
    }

    fn score(&self) -> f64 {
        let raw_score_point = self.raw_score as i64 * MAX_TURN as i64;
        let power_point = self.power as i64 * (MAX_TURN - self.turn) as i64;
        let mut partial_power_point = 0.0;

        for enemy in self.enemies.iter().flatten() {
            let ratio = (enemy.init_hp - enemy.hp) as f64 / enemy.init_hp as f64;
            partial_power_point += enemy.power as f64 * ratio;
        }

        raw_score_point as f64 + power_point as f64 + partial_power_point
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Enemy {
    hp: u32,
    init_hp: u32,
    power: u32,
}

impl Enemy {
    fn new(hp: u32, power: u32) -> Self {
        Self {
            hp,
            init_hp: hp,
            power,
        }
    }

    fn damage(&mut self, attack: u32) {
        self.hp = self.hp.saturating_sub(attack);
    }
}

fn main() {
    let mut state = State::new();
    let mut turn = 0;

    while let Some(enemies) = read_spawns() {
        let mut current_states = vec![None; WIDTH];
        current_states[state.column] = Some((state.clone(), C));
        let simulation_len = DEFAULT_SIMULATION_LEN.min(MAX_TURN - turn);

        for iter in 0..simulation_len {
            let mut next_states: Vec<Option<(State, usize)>> = vec![None; WIDTH];

            for state in current_states.iter() {
                if let Some((state, first_dir)) = state {
                    for &dir in &[L, C, R] {
                        let mut state = state.clone();
                        let is_alive = state.progress_turn(&enemies, dir);

                        if !is_alive {
                            continue;
                        }

                        let next_col = state.column;

                        if next_states[next_col]
                            .as_ref()
                            .map_or(std::f64::MIN, |s| s.0.score())
                            < state.score()
                        {
                            let dir = if iter == 0 { dir } else { *first_dir };
                            next_states[next_col] = Some((state, dir));
                        }
                    }
                }
            }

            current_states = next_states;
        }

        let mut best_score = std::f64::MIN;
        let mut best_dir = C;

        for s in current_states.iter() {
            if let Some((state, dir)) = s {
                if best_score.change_max(state.score()) {
                    best_dir = *dir;
                }
            }
        }

        eprintln!("power: {}", state.power);
        eprintln!("score: {}", state.raw_score);
        eprintln!("turn: {}", turn + 1);
        eprintln!("best_score: {}", best_score);
        eprintln!("");

        write_direction(best_dir);
        state.progress_turn(&enemies, best_dir);
        turn += 1;

        if turn == MAX_TURN {
            break;
        }
    }
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
