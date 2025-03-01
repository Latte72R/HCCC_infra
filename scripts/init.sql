-- ENUM型の定義
CREATE TYPE JudgeResult AS ENUM (
    'AC',
    'WA',
    'WC',
    'AE',
    'LE',
    'RE',
    'TLE',
    'Pending',
    'SystemError'
);

CREATE TYPE Arch AS ENUM (
    'x8664',
    'riscv'
);

CREATE TYPE TestTarget AS ENUM (
    'ExitCode',
    'StdOut',
    'NoTestCase'
);

-- ユーザ情報テーブル
CREATE TABLE accounts (
    id serial primary key,
    name text unique not null,
    password text not null
);

-- セッションテーブル
CREATE TABLE sessions (
    session_key text primary key,
    user_id integer REFERENCES accounts(id) ON UPDATE NO ACTION ON DELETE CASCADE,
    created_at timestamptz not null
);

-- 問題テーブル（システムで参照するための列も含む）
CREATE TABLE problems (
    id serial primary key,
    title text not null,
    statement text not null,
    code text not null,
    input_desc text,
    output_desc text,
    arch Arch,
    test_target TestTarget,
    is_wrong_code boolean,
    error_line_number integer,
    score integer not null
);

-- テストケーステーブル
CREATE TABLE testcases (
    id serial primary key,
    problem_id integer REFERENCES problems(id) ON UPDATE NO ACTION ON DELETE CASCADE,
    input text,
    expect text
);

-- 提出情報テーブル
CREATE TABLE submits (
    id serial primary key,
    user_id integer REFERENCES accounts(id) ON UPDATE NO ACTION ON DELETE CASCADE,
    problem_id integer REFERENCES problems(id) ON UPDATE NO ACTION ON DELETE CASCADE,
    time timestamptz not null,
    asm text not null,
    error_message text not null,
    is_ce boolean not null,
    error_line_number integer,
    result JudgeResult not null
);

-- 各問題のINSERT文

-- 問題0: Return 42
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    0,
    'Return 42',
    'exitcodeで42を返すプログラムをコードの通りにコンパイルしてください．チュートリアルを読めばできるはず．',
    'int main(void) {
    return 42;
}',
    '無し',
    'exitcodeで出力',
    'x8664',
    'ExitCode',
    false,
    null,
    100
);

-- 問題1: Addition of constants
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    1,
    'Addition of constants',
    '定数の足し算をするプログラムをコードの通りにコンパイルしてください．練習も兼ねて答えを直書きしないで実際に足す命令を使いましょう．',
    'int main(void) {
    return 5 + 2;
}',
    '無し',
    'exitcodeで出力',
    'x8664',
    'ExitCode',
    false,
    null,
    100
);

-- 問題2: Subtraction of constants
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    2,
    'Subtraction of constants',
    '定数の引き算をするプログラムをコードの通りにコンパイルしてください．',
    'int main(void) {
    return 255 - 55;
}',
    '無し',
    'exitcodeで出力',
    'x8664',
    'ExitCode',
    false,
    null,
    100
);

-- 問題3: Four arithmetic operations
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    3,
    'Four arithmetic operations',
    '定数の四則演算をするプログラムをコードの通りにコンパイルしてください．掛け算や割り算はやや面倒ですが，これもいい練習です．',
    'int main(void) {
    return 2 * 3 - (8 / 5);
}',
    '無し',
    'exitcodeで出力',
    'x8664',
    'ExitCode',
    false,
    null,
    100
);

-- 問題4: Local variable
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    4,
    'Local variable',
    'ローカル変数を含むプログラムをコードの通りにコンパイルしてください.',
    'int main(void) {
    int a = 3;
    return a;
}',
    '無し',
    'exitcodeで出力',
    'x8664',
    'ExitCode',
    false,
    null,
    100
);

-- 問題5: return else（誤ったコード）
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    5,
    'return else',
    '以下のプログラムをコードの通りにコンパイルしてください.',
    'int main(void) {
    return else;
}',
    '無し',
    'exitcodeで出力',
    'x8664',
    'NoTestCase',
    true,
    2,
    100
);

-- 問題6: Global variable
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    6,
    'Global variable',
    'グローバル変数を含むプログラムをコードの通りにコンパイルしてください',
    'int a = 0;
void add_two(void) {
    a += 2;
}

int main(void) {
    add_two();
    return a;
}',
    '無し',
    'exitcodeで出力',
    'x8664',
    'ExitCode',
    false,
    null,
    100
);

-- 問題7: Call function with args
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    7,
    'Call function with args',
    '引数ありの関数呼び出しを含むプログラムをコードの通りにをコンパイルしてください.',
    'int add(int x, int y){
    return x + y;
}

int main(void) {
    return add(5, 4);
}',
    '無し',
    'exitcodeで出力',
    'x8664',
    'ExitCode',
    false,
    null,
    100
);

-- 問題8: String
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    8,
    'String',
    '文字列を含むプログラムをコードの通りにコンパイルしてください。',
    'int main(void) {
    char array[6] = "Hello";
    return array[2];
}',
    '無し',
    'exitcodeで出力',
    'x8664',
    'ExitCode',
    false,
    null,
    100
);

-- 問題9: return 23
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    9,
    'return 23',
    '23を返すプログラムをコードの通りにコンパイルしてください。',
    'int main{void}(
    return 23;
)',
    '無し',
    'exitcodeで出力',
    'x8664',
    'ExitCode',
    false,
    null,
    100
);

-- 問題10: Sum
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    10,
    'Sum',
    '1~nの合計を計算するプログラムをコードの通りにコンパイルしてください．最適化？おぬしにはまだ早い．',
    'int main(void) {
    int n = 10, sum = 0;
    for (int i=1, i<=n; i++) {
        sum += i;
    }
    return sum;
}',
    '無し',
    'exitcodeで出力',
    'x8664',
    'ExitCode',
    false,
    null,
    100
);

-- 問題11: Hello,world!
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    11,
    'Hello,world!',
    'ここまで長い道のりでしたね．ようやくHelloWorldです．標準出力に出力するコードをコンパイルしてください．標準出力に出す問題ではexit codeに0を返すのをお忘れなく．',
    '#include <stdio.h>

int main(void) {
    printf("Hello,world!");
    return 0;
}',
    '無し',
    '標準出力',
    'x8664',
    'StdOut',
    false,
    null,
    100
);

-- 問題12: Echo
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    12,
    'Echo',
    '入力された文字をおうむ返しするコードをコンパイルしてください.',
    '#include <stdio.h>

int main(void) {
    char str[30];
    scanf("%s", &str);
    printf("%s", str);
    return 0;
}',
    '1 <= len(s) <= 29',
    '標準出力、入力文字列と同じ',
    'x8664',
    'StdOut',
    false,
    null,
    200
);

-- 問題13: FizzBuzz
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    13,
    'FizzBuzz',
    '入門にぴったりなFizzBuzzですが，アセンブリで書くのはやや大変．',
    '#include <stdio.h>
int typedef INT;

int main(void) {
    INT d;
    scanf("%d", &d);
    for (int i=1;i<=d;i++) {
        if (i%15 == 0) {
            printf("FizzBuzz");
        } else if (i%3==0) {
            printf("Fizz");
        } else if (i%5 == 0) {
            printf("Buzz");
        } else {
            printf("%d", i);
        }
    }
    return 0;
}',
    '1 <= d <= 20',
    '標準出力',
    'x8664',
    'StdOut',
    false,
    null,
    200
);

-- 問題14: Increment, Decrement
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    14,
    'Increment, Decrement',
    'インクリメントとデクリメントを含むプログラムをコンパイルしてください.',
    '#include <stdio.h>

int main(void) {
    int d, e, f;
    scanf("%d %d %d", &d, &e, &f);
    printf("%d", d++ + ++e - --f);
    return 0;
}',
    '1 <= d, e, f <= 100',
    '標準出力',
    'x8664',
    'StdOut',
    false,
    null,
    200
);

-- 問題15: Assign
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    15,
    'Assign',
    '代入するプログラムをコンパイルしてください。',
    '#include <stdio.h>

int main(void) {
    int a, b, c;
    scanf("%d", &c);
    a = (b = 2) = c ;
    printf("%d", a);
    return 0;
}',
    '1 <= c <= 100',
    '標準出力',
    'x8664',
    'StdOut',
    true,
    null,
    300
);

-- 問題16: fibonacci
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    16,
    'fibonacci',
    'フィボナッチ数列を計算するプログラムをコンパイルしてください。',
    '#include <stdio.h>

int fib(int n) {
    if (n == 0 || n == 1) return 1;
    return fib(n-1) + fib(n-2);
}

int main(void) {
    int d;
    scanf("%d", &d);
    printf("%d", fib(d));
    return 0;
}',
    '1 <= d <= 20',
    '標準出力',
    'x8664',
    'StdOut',
    false,
    null,
    300
);

-- 問題17: ternary operator
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    17,
    'ternary operator',
    '3項演算子を含むコードをコンパイルしてみましょう．',
    '#include <stdio.h>

struct S {
    int m;
};

int main(void) {
    struct S s1 = {1}, s2 = {2};
    int d;
    scanf("%d", &d);
    printf("%d", (d == 1 ? s1 : s2).m);
    return 0;
}',
    '1 <= d <= 5',
    '標準出力',
    'x8664',
    'StdOut',
    false,
    null,
    300
);

-- 問題18: d.e.f
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    18,
    'd.e.f',
    '3つの変数を使ったプログラムをコンパイルしてみよう！',
    '#include <stdio.h>

int main(void) {
    int *d, e, **f;
    e = 10;
    d = &e;
    f = &d;
    printf("%d", e**d***f);
    return 0;
}',
    '無し',
    '標準出力',
    'x8664',
    'StdOut',
    false,
    null,
    400
);

-- 問題19: Switch ?
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    19,
    'Switch ?',
    'switch文(?)をコンパイルしてみよう!最適化せずにこのコードのままコンパイルしてね！',
    '#include <stdio.h>
#include <stdlib.h>

int main(void){
    char *psz1 = calloc(100, sizeof(char));
    char *psz2 = "abcdefghijklmnopqrstuvwxyz";
    char *to   = psz1;
    char *from = psz2;
    int  count = 26;

    switch (count % 8) {
        case 0:  do {  *to++ = *from++;
        case 7:        *to++ = *from++;
        case 6:        *to++ = *from++;
        case 5:        *to++ = *from++;
        case 4:        *to++ = *from++;
        case 3:        *to++ = *from++;
        case 2:        *to++ = *from++;
        case 1:        *to++ = *from++;
        } while ((count -= 8) > 0);
    }

    printf("%s", psz1);
    return 0;
}',
    '無し',
    'StdOut',
    'x8664',
    'StdOut',
    false,
    null,
    500
);

-- 問題20: Four arithmetic operations2
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    20,
    'Four arithmetic operations2',
    '最適化で全部消したりしないでね！',
    '#include<stdio.h>
int main(void){
    int a=2;int*p=&a;
    int twelve=10+*p;
    int eight =10-*p;
    int twenty=10**p;
    int five  =10/*p;
}',
    '無し',
    'exitcodeで出力',
    'x8664',
    'ExitCode',
    true,
    null,
    500
);

-- 問題21: 50 ** 2
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    21,
    '50 ** 2',
    '最近はいろんな言語でべき乗ができて便利ですよね．',
    '#include <stdio.h>
int main(void){
    if( 50 ** "2" == 2500 ) {
            printf("C language has a power operator!?");
    } else {
            printf("C language does not have a power operator...");
    }
    return 0;
}',
    '無し',
    '標準出力',
    'x8664',
    'StdOut',
    true,
    null,
    500
);

-- 問題22: Thank you seccamp
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, arch, test_target, is_wrong_code, error_line_number, score)
VALUES (
    22,
    'Thank you seccamp',
    'ありがとう，セキュリティ・キャンプ．',
    '#include <stdio.h>
int main(void) {
    printf("Security camp is a very exciting event!\n");
    printf("For more info, please visit:\n");
    https://www.ipa.go.jp/jinzai/security-camp/
    return 0;
}',
    '無し',
    '標準出力',
    'x8664',
    'StdOut',
    true,
    null,
    1000
);
