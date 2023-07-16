#include <iostream>
#include <vector>
#include <queue>
#include <random>
#include <string>
#include <sstream>
#include <fstream>
#include <cassert>

constexpr int width = 25;  // フィールドの幅
constexpr int height = 60; // フィールドの高さ
constexpr int min_P = 1;   // 敵機の最小出現確率
constexpr int max_P = 8;  // 敵機の最大出現確率
constexpr int max_turn = 1000; // ゲームの最大ターン数

std::uniform_int_distribution<> cent(1, 100);
std::uniform_int_distribution<> decide_P(min_P, max_P);

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
	// シード値の決定
	if(argc < 3) {
		std::cerr << "two parameters needed" << std::endl;
		std::cerr << "input the number of files and start number on command line" << std::endl;
		return 0;
	}
	int number_of_files = std::stoi(argv[1]), start_number = std::stoi(argv[2]);
	uint64_t seeds[number_of_files];
	for(int i = 0; i < number_of_files; i++){
		seeds[i] = i + start_number;
	}

	// 入力生成開始
	int cnt = 0;
	std::vector<std::mt19937_64> mt;
	for(int i = 0; i < number_of_files; i++){
		mt.emplace_back(seeds[i]);
	}
	while(cnt < number_of_files){
		// 初期化

		// ファイル名の決定
		std::stringstream ss;
		std::string num = std::to_string(start_number + cnt);
		int siz = num.size();
		for(int i = 0; i < 4 - siz; i++){
			num = '0' + num;
		}
		ss << num << ".txt";
		const char* in_fname = ss.str().c_str();

		// テストケースの生成
		std::vector<int> P(width);
		for(int i = 0; i < width; i++) {
			int poss = decide_P(mt[cnt]);
			P[i] = poss;
		}
		std::vector<std::vector<Enemy>> enemies(max_turn + 1);
		for(int T = 1; T <= max_turn; T++) {
			std::normal_distribution<> rand_hp(7.5 + 0.15 * T, 1.5 + 0.03 * T);
			for(int x = 0; x < width; x++) {
				int rnd = cent(mt[cnt]);
				if(rnd <= P[x]) {
					double d_hp = rand_hp(mt[cnt]);
					std::normal_distribution<> rand_power(d_hp * 0.8, d_hp * 0.1);
					double d_power = rand_power(mt[cnt]);
					int hp = std::max(1, (int)d_hp);
					int power = std::max(0, (int)d_power);
					enemies[T].emplace_back(hp, power, x);
				}
			}
		}

		// ファイルへの書き込み
		std::ofstream ofs(in_fname);
		for(int i = 0; i < width - 1; i++) {
			ofs << P[i] << " ";
		}
		ofs << P[width - 1] << std::endl;
		for(int turn = 1; turn <= max_turn; turn++) {
			int n = enemies[turn].size();
			ofs << n << std::endl;
			for(int i = 0; i < n; i++) {
				auto& e = enemies[turn][i];
				ofs << e.h << " " << e.p << " " << e.x << std::endl;
			}
		}

		cnt++;
	}

	return 0;
}
