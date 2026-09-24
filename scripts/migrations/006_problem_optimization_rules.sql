-- Keep existing databases in sync with the current problem text.

UPDATE problems
SET statement = '定数の足し算をするプログラムをコードの通りにコンパイルしてください．この問題では恣意的な最適化は禁止です．練習も兼ねて答えを直書きせず，実際に足す命令を使いましょう．'
WHERE id = 1;

UPDATE problems
SET statement = '定数の引き算をするプログラムをコードの通りにコンパイルしてください．この問題では恣意的な最適化は禁止です．'
WHERE id = 2;

UPDATE problems
SET statement = '定数の四則演算をするプログラムをコードの通りにコンパイルしてください．この問題では恣意的な最適化は禁止です．掛け算や割り算はやや面倒ですが，これもいい練習です．'
WHERE id = 3;

UPDATE problems
SET statement = 'ローカル変数を含むプログラムをコードの通りにコンパイルしてください．この問題では恣意的な最適化は禁止です．'
WHERE id = 4;

UPDATE problems
SET statement = '関数を含むプログラムをコードの通りにコンパイルしてください．この問題では恣意的な最適化は禁止です．'
WHERE id = 6;

UPDATE problems
SET statement = 'グローバル変数を含むプログラムをコードの通りにコンパイルしてください．この問題では恣意的な最適化は禁止です．'
WHERE id = 7;

UPDATE problems
SET statement = '引数ありの関数呼び出しを含むプログラムをコードの通りにコンパイルしてください．この問題では恣意的な最適化は禁止です．'
WHERE id = 8;

UPDATE problems
SET statement = '文字列を含むプログラムをコードの通りにコンパイルしてください．この問題では恣意的な最適化は禁止です．',
    code = 'int main() {
    char *array = "Hello";
    return array[2];
}'
WHERE id = 9;

UPDATE problems
SET statement = '1~nの合計を計算するプログラムをコードの通りにコンパイルしてください．この問題では恣意的な最適化は禁止です．'
WHERE id = 11;

UPDATE problems
SET statement = '配列の要素を全て掛け合わせた結果を17で割った余りを出力するプログラムをコンパイルしてください．この問題では恣意的な最適化は禁止です．'
WHERE id = 17;
