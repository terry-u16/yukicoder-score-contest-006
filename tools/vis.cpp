# include <Siv3D.hpp>

constexpr int32 inf = 10000;
constexpr int32 height = 60;
constexpr int32 width = 25;
constexpr int32 max_turn = 1000;
constexpr int32 level_up = 100;

bool is_out(int32 y, int32 x) {
    if(y < 0 or y >= height) return true;
    if(x < 0 or x >= width) return true;
    return false;
}

// 敵機の情報
struct Enemy {
    int32 init_hp;
    int32 h, p;
    int32 x, y;
    bool will_disappear;
    Enemy(int32 h, int32 p, int32 x) : h(h), p(p), x(x) {
        init_hp = h;
        y = height - 1;
        will_disappear = false;
    }
};
using EnemyStatus = Optional<Enemy>;

// プレイヤーの情報
struct Player{
    int32 x, y;
    int32 level, power;
    int32 score;
    char32 next_move;
    bool game_over;
    Player(int32 x, int32 y) : x(x), y(y) {
        level = 1;
        power = 0;
        score = 0;
        next_move = 'S';
        game_over = false;
    }
    void destroy_enemy(Enemy& e) {
        score += e.init_hp;
        power += e.p;
        level = 1 + power / level_up;
    }
};

// フィールドの状態を更新する関数 -1 : invalid, 0 : game over, 1 : continue
int32 Update(Grid<EnemyStatus>& field, const Array<Enemy>& enemy_appear, Player& player) {
    // 前ターン破壊された敵機の消去
    for(auto y : step(height)) {
        for(auto x : step(width)) {
            if(field[y][x] and field[y][x]->will_disappear) {
                field[y][x].reset();
            }
        }
    }
    bool game_over = false;
    // 敵機の移動
    for(auto y : step(height)) {
        for(auto x : step(width)) {
            if(not field[y][x]) continue;
            field[y][x]->y -= 1;
            
            if(y == 0) {
                field[y][x].reset();
            }
            else {
                field[y-1][x] = field[y][x];
                field[y][x].reset();
            }
            // 自機に衝突したらゲーム終了
            if(y - 1 == player.y and x == player.x) {
                game_over = true;
            }
        }
    }
    // 敵機の出現
    int n = enemy_appear.size();
    for(auto new_enemy : enemy_appear) {
        field[height-1][new_enemy.x] = new_enemy;
    }
    if(game_over) {
        player.game_over = true;
        return 0;
    }
    // 自機の移動
    if(player.next_move == 'S'){
        // 何もしない
    }
    else if(player.next_move == 'L') {
        player.x -= 1;
    }
    else if(player.next_move == 'R') {
        player.x += 1;
    }
    else {
        return -1;
    }
    player.x += width; player.x %= width;
    // 敵機に衝突したらゲーム終了
    if(field[player.y][player.x]) {
        game_over = true;
    }
    if(game_over) {
        player.game_over = true;
        return 0;
    }
    // 自機の攻撃
    int32 py = player.y + 1, px = player.x;
    while(not is_out(py, px)) {
        if(field[py][px]) {
            field[py][px]->h -= player.level;
            // 体力が0以下になったら破壊予約をしておく
            if(field[py][px]->h <= 0) {
                player.destroy_enemy(field[py][px].value());
                field[py][px]->will_disappear = true;
            }
            break;
        }
        py += 1;
    }

    return 1;
}

// フィールドの状態を画像化する関数
void CopyToImage(const Grid<EnemyStatus>& field, const Player& player, Image& image) {
    for (auto y : step(image.height())) {
        for (auto x : step(image.width())) {
            if(field[y][x] and (not field[y][x]->will_disappear)) {
                image[height-1-y][x] = Palette::Red;
            }
            else if(y == player.y and x == player.x) {
                image[height-1-y][x] = Palette::Lime;
            }
            else {
                image[height-1-y][x] = Palette::Black;
            }
        }
    }
}

// 自機による攻撃の描画
void DrawLaser(const Grid<EnemyStatus>& field, const Player& player, int cell_size) {
    if(player.game_over) return;
    int32 py = player.y + 1, px = player.x;
    while(not is_out(py, px)) {
        int32 y = height - 1 - py, x = px;
        int32 r = cell_size / 2;
        if(field[py][px]) {
            Circle{x * cell_size + r, y * cell_size + r, r}.draw(Palette::Orange);
            return;
        }
        Rect{Arg::center(x * cell_size + r, y * cell_size + r), 2, cell_size}
            .draw(Palette::Orange);
        py += 1;
    }
}

void Main(){
    Window::SetTitle(U"Tool-assisted Shooting");
    constexpr int32 window_width = 600;
    constexpr int32 window_height = 720;
    constexpr int32 cell_size = window_height / height;
    Window::Resize(Size{window_width, window_height});
    
    // 文字描画用
    Font font{20};
    Rect enemy_cell{350, 500, 10, 10};
    Rect player_cell{440, 500, 10, 10};
    
    int32 turn = 0;
    double turn_ratio = 0.0;
    
    // 不正な入出力がないか
    int32 invalid_turn = inf;
    
    // 敵機の情報を保存する配列
    Array<Grid<EnemyStatus>> fields;
    
    // プレイヤーの情報を保存する配列
    Array<Player> players;

    // フィールドの状態を可視化するための画像
    Image image{ width, height, Palette::Black };

    // 動的テクスチャ
    DynamicTexture texture{ image };

    Stopwatch stopwatch{ StartImmediately::Yes };

    // 自動再生
    bool autoStep = false;

    // 更新頻度
    double speed = 0.5;

    // 画像の更新の必要があるか
    bool updated = true;
    
    // ファイル読み込み用
    TextReader input_reader{U"input.txt"};
    TextReader output_reader{U"output.txt"};
        
    // 初期状態
    Grid<EnemyStatus> field(width, height);
    fields << field;
    Player player(12, 0);
    players << player;
    Array<Array<String>> Comments(max_turn + 1);
    
    String in_line, out_line;
    bool failed = false;
    if(not input_reader) {
        System::MessageBoxOK(U"Failed to open the input file");
        System::Exit();
        failed = true;
    }
    if(not output_reader) {
        System::MessageBoxOK(U"Failed to open the output file");
        System::Exit();
        failed = true;
    }
    input_reader.readLine(in_line); // 最初の一行は利用しない
    
    // 各ターンの状態を保存
    int32 last_turn = 0;
    for(auto i : step(max_turn)) {
        if(failed) break;
        last_turn++;
        
        Array<Enemy> new_enemies;
        input_reader.readLine(in_line);
        int n = Parse<int32>(in_line);
        
        for(auto j : step(n)) {
            input_reader.readLine(in_line);
            Array<String> e = in_line.split(' ');
            int32 h = Parse<int32>(e[0]);
            int32 p = Parse<int32>(e[1]);
            int32 x = Parse<int32>(e[2]);
            new_enemies << Enemy(h, p, x);
        }
        
        while(output_reader.readLine(out_line)) {
            if(out_line[0] == '#') {
                Comments[i] << out_line;
                continue;
            }
            else break;
        }
        char32 c = out_line[0];
        player.next_move = c;
        int32 res = Update(field, new_enemies, player);
        
        fields << field;
        players << player;
        
        if(res == -1) {
            invalid_turn = last_turn;
            break;
        }
        else if(res == 0) {
            break;
        }
    }

    while (System::Update()){
        
        // グリッド説明
        enemy_cell.draw(Palette::Red);
        font(U" : 敵機").draw(360, 490);
        
        player_cell.draw(Palette::Lime);
        font(U" : プレイヤー").draw(450, 490);
        
        // 現在のスコア
        if(invalid_turn < turn){
            font(U"Score = 0").draw(360, 70);
        }
        else{
            font(U"Score = ", players[turn].score).draw(360, 20);
        }
        // 現在のパワー
        font(U"Power = ", players[turn].power).draw(360, 50);
        // 現在のレベル
        font(U"Level = ", players[turn].level).draw(360, 80);
        
        // 更新頻度変更スライダー
        SimpleGUI::SliderAt(U"Speed", speed, 1.0, 0.1, Vec2{450, 160}, 80, 100);

        // 一時停止 / 再生ボタン
        if (SimpleGUI::ButtonAt(autoStep ? U"Pause" : U"Run", Vec2{450, 200}, 180))
        {
            autoStep = !autoStep;
        }
        
        // ターン数設定用スライダー
        bool turn_changed = SimpleGUI::Slider(turn_ratio, Vec2{360, 270}, 180);
        if(turn_changed) {
            turn = last_turn * turn_ratio;
            updated = true;
        }
        font(U"Turn = ", turn).draw(360, 240);

        // 1 ステップ進めるボタン、または更新タイミングの確認
        if (SimpleGUI::ButtonAt(U"Step", Vec2{405, 330}, 90)
            || (autoStep && stopwatch.sF() >= (speed * speed)))
        {
            if(turn < last_turn){
                turn++;
                updated = true;
                stopwatch.restart();
            }
        }
        // 1 ステップ戻すボタン
        if (SimpleGUI::ButtonAt(U"Back", Vec2{495, 330}, 90)){
            if(turn > 0){
                turn--;
                updated = true;
            }
        }

        // 画像の更新
        if (updated and invalid_turn >= turn){
            CopyToImage(fields[turn], players[turn], image);
            texture.fill(image);
            updated = false;
        }

        // 画像をフィルタなしで拡大して表示
        {
            ScopedRenderStates2D sampler{ SamplerState::ClampNearest };
            texture.scaled(12).draw();
        }

        // グリッドの表示
        for (auto i : step(height + 1)) {
            Rect{ 0, i * cell_size, width * cell_size, 1 }.draw(ColorF{ 0.4 });
        }
        for(auto i : step(width + 1)) {
            Rect{ i * cell_size, 0, 1, height * cell_size }.draw(ColorF{ 0.4 });
        }
        
        // コメントの表示
        Rect{320, 360, 260, 120}.drawFrame(2, 0, Palette::White);
        for(auto i : step(Comments[turn].size())){
            font(Comments[turn][i]).draw(325, 365 + i * 20);
        }
        
        // カーソルを合わせた際の敵機の情報表示
        if(Rect{0, 0, width * cell_size, height * cell_size}.mouseOver()){
            Rect{Cursor::Pos() / cell_size * cell_size, cell_size}
                .drawFrame(2, 0, Palette::White);
            int32 cursor_y = height - 1 - Cursor::Pos().y / cell_size;
            int32 cursor_x = Cursor::Pos().x / cell_size;
            font(U"(", cursor_x, U", ", cursor_y, U")")
                .draw(window_width - 100, window_height - 100);
            
            if(fields[turn][cursor_y][cursor_x]) {
                EnemyStatus ptr = fields[turn][cursor_y][cursor_x];
                font(U"initial hp = ", ptr->init_hp)
                    .draw(window_width - 200, window_height - 180);
                font(U"hp = ", ptr->h)
                    .draw(window_width - 200, window_height - 160);
                font(U"power = ", ptr->p)
                    .draw(window_width - 200, window_height - 140);
            }
            
        }
        
        if(invalid_turn >= turn){
            DrawLaser(fields[turn], players[turn], cell_size);
        }
        if(invalid_turn != 0 and invalid_turn != inf){
            font(U"Invalid action in turn ", invalid_turn, U" : ",
                 players[invalid_turn].next_move)
            .draw(window_width - 275, window_height - 50);
        }
        else if(turn == last_turn) {
            font(U"Game Over").draw(window_width - 275, window_height - 50);
        }
    }
}
