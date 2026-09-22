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

CREATE TABLE accounts -- ユーザ
(
    id serial primary key,
    name text unique not null,
    password text not null
);

CREATE TABLE sessions
(
    session_key text primary key,
    user_id integer REFERENCES accounts(id) ON UPDATE NO ACTION ON DELETE CASCADE,
    created_at timestamptz not null
);

CREATE TABLE problems -- 問題
(
    id serial primary key,
    title text not null,
    statement text not null,
    code text not null,
    input_desc text,
    output_desc text,
    test_target TestTarget,
    is_wrong_code bool,
    error_line_number integer,
    score integer not null
);

CREATE TABLE testcases -- テストケース
(
    id serial primary key,
    problem_id integer REFERENCES problems(id) ON UPDATE NO ACTION ON DELETE CASCADE,
    input text,
    expect text
);

CREATE TABLE submits -- submit
(
    id serial primary key,
    user_id integer REFERENCES accounts(id) ON UPDATE NO ACTION ON DELETE CASCADE,
    problem_id integer REFERENCES problems(id) ON UPDATE NO ACTION ON DELETE CASCADE,
    time timestamptz not null,
    asm text not null,
    error_message text not null,
    is_ce boolean not null,
    error_line_number integer,
    result JudgeResult not null,
    -- Architecture chosen by the submitter at submit time.
    arch Arch not null DEFAULT 'x8664',
    -- Multi-replica judge claim lease. NULL means unclaimed.
    claimed_at timestamptz,
    claimed_by text
);

CREATE INDEX IF NOT EXISTS idx_submits_pending_claim
    ON submits (result, claimed_at, time) WHERE result = 'Pending';

CREATE TABLE admin_judge_audit (
    id bigserial primary key,
    submission_id integer not null REFERENCES submits(id),
    admin_user_id integer not null REFERENCES accounts(id),
    previous_result text not null,
    previous_error_message text not null,
    new_result text not null,
    new_error_message text not null,
    changed_at timestamptz not null DEFAULT now()
);

-- Contest period, editable by admins via /api/admin/contest.
-- Seeded from CONTEST_BEGIN/CONTEST_END at web startup when empty.
CREATE TABLE contest_config (
    key text primary key,
    value text not null
);

-- Default administrator: admin / P@ssw0rd.
-- Password column stores hex(SHA256(password)).
INSERT INTO accounts (id, name, password) VALUES (
    1,
    'admin',
    'b03ddf3ca2e714a6548e7495e2a03f5e824eaac9837cd7f159c67b90fb4b7342'
) ON CONFLICT (id) DO NOTHING;
SELECT setval('accounts_id_seq', (SELECT greatest(max(id), 1) FROM accounts));

-- Seed rows use explicit ids; sync all serial sequences afterwards.
-- 各問題のINSERT文
INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    0,
    'Return 72',
    'exitcodeで72を返すプログラムをコードの通りにコンパイルしてください．チュートリアルを読めばできるはず．',
    'int main() {
    return 72;
}',
    '無し',
    'exitcodeで出力',
    'ExitCode',
    false,
    null,
    100
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    0,
    '',
    '72'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    1,
    'Addition of constants',
    '定数の足し算をするプログラムをコードの通りにコンパイルしてください．練習も兼ねて答えを直書きしないで実際に足す命令を使いましょう．',
    'int main() {
    return 5 + 2;
}',
    '無し',
    'exitcodeで出力',
    'ExitCode',
    false,
    null,
    100
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    1,
    '',
    '7'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    2,
    'Subtraction of constants',
    '定数の引き算をするプログラムをコードの通りにコンパイルしてください．',
    'int main() {
    return 255 - 55;
}',
    '無し',
    'exitcodeで出力',
    'ExitCode',
    false,
    null,
    100
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    2,
    '',
    '200'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    3,
    'Four arithmetic operations',
    '定数の四則演算をするプログラムをコードの通りにコンパイルしてください．掛け算や割り算はやや面倒ですが，これもいい練習です．',
    'int main() {
    return 22 * 4 - 48 / 3;
}',
    '無し',
    'exitcodeで出力',
    'ExitCode',
    false,
    null,
    100
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    3,
    '',
    '72'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    4,
    'Local variable',
    'ローカル変数を含むプログラムをコードの通りにコンパイルしてください．最適化しないでください．',
    'int main() {
    int a = 3;
    return a;
}',
    '無し',
    'exitcodeで出力',
    'ExitCode',
    false,
    null,
    100
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    4,
    '',
    '3'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    6,
    'Call function',
    '関数を含むプログラムをコードの通りにコンパイルしてください.',
    'int five() {
    return 5;
}
int main() {
    return five();
}',
    '無し',
    'exitcodeで出力',
    'ExitCode',
    false,
    null,
    100
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    6,
    '',
    '5'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    7,
    'Global variable',
    'グローバル変数を含むプログラムをコードの通りにコンパイルしてください',
    'int a = 9;
void add_two() {
    a += 2;
}

int main() {
    add_two();
    return a;
}',
    '無し',
    'exitcodeで出力',
    'ExitCode',
    false,
    null,
    100
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    7,
    '',
    '11'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    8,
    'Call function with args',
    '引数ありの関数呼び出しを含むプログラムをコードの通りにをコンパイルしてください.',
    'int add(int x, int y) {
    return x + y;
}

int main() {
    return add(5, 4);
}',
    '無し',
    'exitcodeで出力',
    'ExitCode',
    false,
    null,
    100
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    8,
    '',
    '9'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    9,
    'String',
    '文字列を含むプログラムをコードの通りにコンパイルしてください。',
    'int main() {
    char array[6] = "Hello";
    return array[2];
}',
    '無し',
    'exitcodeで出力',
    'ExitCode',
    false,
    null,
    100
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    9,
    '',
    '108'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    10,
    'Hello,world!',
    'ここまで長い道のりでしたね．ようやくHelloWorldです．標準出力に出力するコードをコンパイルしてください．標準出力に出す問題ではexit codeに0を返すのをお忘れなく．',
    '#include <stdio.h>

int main() {
    printf("Hello, KCS1959!\n");
    return 0;
}',
    '無し',
    '標準出力',
    'StdOut',
    false,
    null,
    100
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    10,
    '',
    E'Hello, KCS1959!\n'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    11,
    'Sum',
    '1~nの合計を計算するプログラムをコードの通りにコンパイルしてください．',
    'int main() {
    int n = 10, sum = 0;
    for (int i=1; i<=n; i++) {
        sum += i;
    }
    return sum;
}',
    '無し',
    'exitcodeで出力',
    'ExitCode',
    false,
    null,
    200
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    11,
    '',
    '55'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    14,
    'Length of string',
    '入力した文字数を数えるプログラムをコンパイルしてください',
    '#include <stdio.h>

int main(void) {
    char str[20];
    int count = 0;
    scanf("%s", str);
    for (int i = 0; str[i] != ''\0''; i++) {
        count++;
    }
    printf("入力された文字数は %d です\n", count);
    return 0;
}',
    '1 <= len(s) <= 19',
    '標準出力',
    'StdOut',
    false,
    null,
    200
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    14,
    'hello',
    '入力された文字数は 5 です\n'
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    14,
    'Hello, world!',
    '入力された文字数は 13 です\n'
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    14,
    'KCS1959',
    '入力された文字数は 7 です\n'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    15,
    'FizzBuzz',
    '入門にぴったりなFizzBuzzですが，アセンブリで書くのはやや大変．',
    '#include <stdio.h>

int main() {
    int d;
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
    'StdOut',
    false,
    null,
    300
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    15,
    '4',
    '12Fizz4'
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    15,
    '15',
    '12Fizz4BuzzFizz78FizzBuzz11Fizz1314FizzBuzz'
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    15,
    '20',
    '12Fizz4BuzzFizz78FizzBuzz11Fizz1314FizzBuzz1617Fizz19Buzz'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    17,
    'Multiplication',
    '配列の要素を全て掛け合わせた結果を17で割った余りを出力するプログラムをコンパイルしてください．',
    'int main() {
  int arr[5] = {5, 2, 4, 3, 7};
  int multi = 1;
  int i;
  for (i = 0; i < 5; i++) {
    multi *= arr[i];
  }
  return multi % 17;
}',
    '無し',
    '標準出力',
    'StdOut',
    false,
    null,
    300
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    17,
    '',
    '6'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    18,
    'Increment, Decrement',
    '最適化で全部消したりしないでね！',
    'int main(void) {
  int a = 3, b = 5, c = 7;
  int *ptr = &b;
  int result = a++ + *ptr * (--c) + ++a;
  return result;
}',
    '無し',
    'exitcodeで出力',
    'ExitCode',
    false,
    null,
    300
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    18,
    '',
    '20'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    19,
    'Fibonacci',
    'フィボナッチ数列を計算するプログラムをコンパイルしてください。',
    '#include <stdio.h>

int fib(int n) {
    if (n == 0 || n == 1) return 1;
    return fib(n-1) + fib(n-2);
}

int main() {
    int d;
    scanf("%d", &d);
    printf("%d", fib(d));
    return 0;
}',
    '1 <= d <= 20',
    '標準出力',
    'StdOut',
    false,
    null,
    300
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    19,
    '7',
    '13'
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    19,
    '10',
    '89'
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    19,
    '20',
    '10946'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    20,
    'Dereference operator',
    '3つの変数を使ったプログラムをコンパイルしてみよう！最適化しないでください．',
    '#include <stdio.h>

int main() {
    int *d, e, **f;
    e = 10;
    d = &e;
    f = &d;
    printf("%d", e**d***f);
    return 0;
}',
    '無し',
    '標準出力',
    'StdOut',
    false,
    null,
    300
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    20,
    '',
    '1000'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    22,
    'Ternary operator',
    '3項演算子を含むコードをコンパイルしてみましょう．  最適化しないでください．',
    '#include <stdio.h>

struct S {
  int m;
  int n;
};

int main(void) {
  struct S s = {5, 3};
  int d;
  scanf("%d", &d);
  int result = d == 1 ? (s.m * 6) : d == 2 ? (s.n * 2) : (s.m + s.n);
  printf("%d\n", result);
  return 0;
}',
    '1 <= d <= 5',
    '標準出力',
    'StdOut',
    false,
    null,
    400
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    22,
    '1',
    '30'
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    22,
    '2',
    '6'
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    22,
    '4',
    '8'
);

INSERT INTO problems (id, title, statement, code, input_desc, output_desc, test_target, is_wrong_code, error_line_number, score)
VALUES (
    23,
    'Prime number',
    'N番目の素数を求めるプログラムをコンパイルしてください．',
    '#include <stdio.h>

int main() {
  int N;
  scanf("%d", &N);
  int prm[30];
  prm[0] = 2;
  int i = 3;
  int n = 1;
  int j;
  while (1) {
    for (j = 0; j < n; j++) {
      if (i % prm[j] == 0)
        break;
      if (j == n - 1) {
        prm[n] = i;
        n++;
        break;
      }
    }
    if (n == N)
      break;
    i++;
  }
  printf("%d\n", prm[N - 1]);
  return 0;
}',
    '1 <= N <= 30',
    '標準出力',
    'StdOut',
    false,
    null,
    400
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    23,
    '3',
    '5'
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    23,
    '16',
    '53'
);

INSERT INTO testcases (problem_id, input, expect) VALUES(
    23,
    '24',
    '89'
);

-- Seed rows use explicit ids; bring serial sequences in sync.
SELECT setval('problems_id_seq', (SELECT greatest(max(id), 1) FROM problems));
SELECT setval('testcases_id_seq', (SELECT greatest(max(id), 1) FROM testcases));
