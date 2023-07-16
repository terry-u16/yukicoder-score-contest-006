#include <iostream>
#include <vector>
#include <string>
#include <fstream>
#include <cassert>

constexpr int width = 25;  // フィールドの幅
constexpr int height = 60; // フィールドの高さ
constexpr int level_up = 100;  // レベルアップに必要なパワー
constexpr int max_turn = 1000; // ゲームの最大ターン数

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

int main(int argc, char* argv[]){

	// テストケースの入力ファイルの input stream
	//std::ifstream input_ifs(argv[1]);
	// テストケースの出力ファイルの input stream = 敵機の情報
	std::ifstream output_ifs(argv[2]);
	// 提出されたコードのファイルの input stream
	//ifstream code_ifs(argv[3]);
	// スコアファイル（スコア問題のみ利用）の output stream
	std::ofstream score_ofs(argv[4]);

	// 敵機出現情報の読み込み
	std::vector<int> P(width);
	for(int i = 0; i < width; i++) {
		output_ifs >> P[i];
	}
	std::vector<std::vector<Enemy>> enemy_appear(max_turn + 1);
	for(int turn = 1; turn <= max_turn; turn++) {
		int n;
		output_ifs >> n;
		for(int i = 0; i < n; i++) {
			int h, p, x;
			output_ifs >> h >> p >> x;
			enemy_appear[turn].emplace_back(h, p, x);
		}
	}

	// コンテスタントへの初期入力

	// リアクティブ開始
	int score = 0, power = 0;
	int player_x = width / 2, player_y = 0;
	std::vector field = std::vector(width, std::vector<Enemy*>(height, nullptr));

	for(int turn = 1; turn <= max_turn; turn++) {
		// 敵機の移動
		for(int x = 0; x < width; x++) {
			for(int y = 0; y < height; y++) {
				if(!field[x][y]) continue;
				field[x][y]->y -= 1;
				// 自機に衝突したらゲーム終了
				if(field[x][y]->x == player_x && field[x][y]->y == player_y) {
					score_ofs << score << std::endl;
					std::cout << -1 << std::endl;
					return 0;
				}
				if(field[x][y]->y < 0) {
					field[x][y] = nullptr;
				}
				else {
					field[x][y-1] = field[x][y];
					field[x][y] = nullptr;
				}
			}
		}
		// 敵機の出現
		int n = enemy_appear[turn].size();
		std::cout << n << std::endl;
		for(auto& itr : enemy_appear[turn]) {
			std::cout << itr.h << " " << itr.p << " " << itr.x << std::endl;
			field[itr.x][height-1] = &itr;
		}
		// 自機の移動
		// 標準入力に提出コードでの標準出力が渡される
		std::string line;
		while(std::getline(std::cin, line)) {
			char c = line[0];
			if(c == '#') continue;
			if(c == 'L') {
				player_x -= 1;
			}
			else if(c == 'R') {
				player_x += 1;
			}
			else if(c == 'S') {
				// 何もしない
			}
			else { // 不正な出力
				std::cout << -1 << std::endl;
				return 1;
			}
			player_x += width;
			player_x %= width;
			break;
		}
		// 敵機がいた場合はゲーム終了
		if(field[player_x][player_y]) {
			score_ofs << score << std::endl;
			std::cout << -1 << std::endl;
			return 0;
		}
		// 自機の攻撃
		for(int y = 1; y < height; y++) {
			if(field[player_x][y]) {
				field[player_x][y]->h -= 1 + power / level_up;
				if(field[player_x][y]->h <= 0) {
					score += field[player_x][y]->init_hp;
					power += field[player_x][y]->p;
					field[player_x][y] = nullptr;
				}
				break;
			}
		}
	}

	// 最大ターン経過したら終了
	score_ofs << score << std::endl;

	return 0;
}
