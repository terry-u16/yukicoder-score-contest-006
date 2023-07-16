#include <iostream>
#include <vector>
#include <fstream>
#include <cassert>
#include <sstream>

constexpr int width = 25;  // フィールドの幅
constexpr int height = 60; // フィールドの高さ
constexpr int level_up = 100;  // レベルアップに必要なパワー
constexpr int max_turn = 1000; // ゲームの最大ターン数

constexpr int testcase = 0;

// 敵機の情報
struct Enemy {
	int init_hp;
	int h, p;
	int x, y;
	Enemy(int h, int p, int x) : h(h), p(p), x(x) {
		init_hp = h;
		y = height - 1;
	}
};

// 自機の情報
struct Player {
	int x, y;
	int score;
	int power, level;
	Player(int x, int y) : x(x), y(y) {
		score = 0;
		power = 0;
		level = 1;
	}
	void destroy_enemy(Enemy* e) {
		score += e->init_hp;
		power += e->p;
		level = 1 + power / level_up;
	}
};

int main(int argc, char* argv[]){
	if(argc < 2) {
		std:: cerr << "input the testcase number on command line" << std::endl;
		return 0;
	}
	// ファイル名の設定
	std::string num = argv[1];
	int siz = num.size();
	for(int i = 0; i < 4 - siz; i++) num = '0' + num;
	std::stringstream input, output;
	input << "in/" << num << ".txt";
	output << "out/" << num << ".txt";
	std::ifstream input_reader(input.str().c_str());
	if(!input_reader) {
		std::cerr << "no such input file" << std::endl;
		return 0;
	}
	std::ifstream output_reader(output.str().c_str());
	if(!output_reader) {
		std::cerr << "no such output file" << std::endl;
		return 0;
	}

	// 敵機出現情報の読み込み
	std::vector<int> P(width);
	for(int i = 0; i < width; i++) {
		input_reader >> P[i];
	}
	std::vector<std::vector<Enemy>> enemy_appear(max_turn + 1);
	for(int T = 1; T <= max_turn; T++) {
		int n;
		input_reader >> n;
		for(int i = 0; i < n; i++) {
			int h, p, x;
			input_reader >> h >> p >> x;
			enemy_appear[T].emplace_back(h, p, x);
		}
	}

	// 出力の読み込み
	std::vector<char> actions;
	std::string line;
	int cnt = 1;
	while(std::getline(output_reader, line)){
		char c = line[0];
		if(c == '#') {
			continue;
		}
		else if(c == 'L'|| c == 'R' || c == 'S') {
			actions.emplace_back(c);
			cnt++;
		}
		else {
			std::cerr << "invalid output in turn " << cnt << " : " << c << std::endl;
			return 0;
		}
	}

	// ゲーム開始
	Player player(12, 0);
	std::vector field = std::vector(height, std::vector<Enemy*>(width, nullptr));

	for(int T = 1; T <= std::min((int)actions.size(), max_turn); T++) {
		// 敵機の移動
		for(int y = 0; y < height; y++) {
			for(int x = 0; x < width; x++) {
				if(!field[y][x]) continue;
				field[y][x]->y -= 1;
				// 自機に衝突したらゲーム終了
				if(field[y][x]->x == player.x && field[y][x]->y == player.y) {
					std::cerr << "turn = " << T << std::endl;
					std::cerr << "score = " << player.score << std::endl;
					return 0;
				}
				if(field[y][x]->y < 0) {
					field[y][x] = nullptr;
				}
				else {
					field[y-1][x] = field[y][x];
					field[y][x] = nullptr;
				}
			}
		}
		// 敵機の出現
		int n = enemy_appear[T].size();
		for(auto& itr : enemy_appear[T]) {
			field[itr.y][itr.x] = &itr;
		}
		// 自機の移動
		char action = actions[T-1];
		if(action == 'L') {
			player.x -= 1;
		}
		else if(action == 'R') {
			player.x += 1;
		}
		else if(action == 'S') {
			// 何もしない
		}
		else { // 不正な出力
			std::cerr << "invalid output" << std::endl;
			return 0;
		}
		player.x += width; player.x %= width;
		// 敵機がいた場合はゲーム終了
		if(field[player.y][player.x]) {
			std::cerr << "turn = " << T << std::endl;
			std::cerr << "score = " << player.score << std::endl;
			return 0;
		}
		// 自機の攻撃
		for(int y = 1; y < height; y++) {
			if(field[y][player.x]) {
				field[y][player.x]->h -= player.level;
				if(field[y][player.x]->h <= 0) {
					player.destroy_enemy(field[y][player.x]);
					field[y][player.x] = nullptr;
				}
				break;
			}
		}
	}

	// 最大ターン経過したら終了
	std::cerr << "turn = " << (int)actions.size() + 1 << std::endl;
	std::cerr << "score = " << player.score << std::endl;

	return 0;
}
