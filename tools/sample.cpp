#include <iostream>

constexpr int width = 25;  // フィールドの幅
constexpr int height = 60; // フィールドの高さ
constexpr int max_turn = 1000; // ゲームの最大ターン数

int main(){
	for(int turn = 1; turn <= max_turn; turn++) {
		// 入力の受け取り
		int n;
		std::cin >> n;
		if(n == -1) return 0;
		for(int i = 0; i < n; i++) {
			int h, p, x;
			std::cin >> h >> p >> x;
		}
		std::cout << 'S' << std::endl;
	}

	return 0;
}
