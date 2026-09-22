# HCCC_Infra

![Logo](/Logo.png)

## 概要

人間 C コンパイラコンテスト(HCCC)とは文字通り競技者自身が C コンパイラとなり C 言語からアセンブリを生成し，その時間と正確さを競う競技です．

与えられるソースコードの中にはコンパイルエラーを出す必要の ある仕様上間違ったものも含まれています．このような場合にはコンパイルエラーと解答する必要があります.

## 起動方法

web_server, judge_server, test_runner, DB の構成です。

HCCC_infra と HCCC_frontend を同じ親ディレクトリに置き、次のコマンドで起動します。
RISC-V ツールチェーンは公開イメージ（`ghcr.io/latte72r/riscv_toolchain_docker:master`）
をそのまま使うため、ローカルでのビルドは不要です。

```bash
docker compose -f docker-compose.yaml -f docker-compose.local.yaml --profile test_runner build test_runner_x8664 test_runner_riscv
docker compose up --build -d
```

独自にツールチェーンをビルドして使う場合は `RISCV_TOOLCHAIN_IMAGE` で上書きできます。

```bash
git clone https://github.com/Latte72R/riscv_toolchain_docker.git ../riscv_toolchain_docker
docker build -t hccc-riscv-toolchain:local ../riscv_toolchain_docker
RISCV_TOOLCHAIN_IMAGE=hccc-riscv-toolchain:local docker compose -f docker-compose.yaml -f docker-compose.local.yaml --profile test_runner build test_runner_riscv
```

また、`.env.example`の環境変数をセットすることが出来ます。

フロントエンドも同じ Compose 構成で起動します。`http://localhost:3000` からアクセスできます。
管理者にするアカウントの ID は `.env` の `ADMIN_USER_IDS` にカンマ区切りで設定してください。
管理画面は `/admin` です。判定修正は変更前後の結果と管理者 ID を
`admin_judge_audit` に記録します。既存 DB でも初回修正時にテーブルを作成します。

## データベース

PostgreSQL 18 (`postgres:18-alpine`) を使います。14 系のボリュームをそのまま
流用はできないため、既存データがある場合はダンプ＆リストアで移行します。
`POSTGRES_IMAGE` でイメージを上書きできます。

```bash
cp .env.example .env   # 資格情報を合わせる
./scripts/migrate-pg14-to-pg18.sh
```

`scripts/init.sql` は新規 DB の初期化専用です。`claimed_at` / `claimed_by`
（複数 judge 用のクレーム列）は `scripts/migrations/002_judge_claim.sql`
でも追加でき、judge 起動時にも自動適用されます。

## 使用技術

- Rust
- axum
- Docker
- postgres

## 開発者向け
以下のコマンドで開発向け環境を立ち上げることができます。

```bash
# 開発向け環境(ホットリロード，dbポート解放)
docker compose -f docker-compose.yaml -f docker-compose.local.yaml up --build
```

また、実際に提出物を実行するtest_runnerは、以下のコマンドでコンテナイメージ作成が行えます。

```bash
docker compose -f docker-compose.yaml -f docker-compose.local.yaml --profile test_runner build test_runner_x8664 test_runner_riscv
```

- x86-64 と RISC-V の test_runner はどちらも x86-64 Linux 上で動く静的リンクの Rust バイナリです。RISC-V の提出プログラムは専用イメージ内のクロス GCC と QEMU user mode で実行します。

infrastructure for [HCCC](https://github.com/Alignof/Human_C_Compiler_Contest)

## Kubernetes に移すとき

- `frontend` は standalone Next.js イメージで、実行時の `HCCC_API_INTERNAL_URL` に `web_server` の ClusterIP Service を指定します。ブラウザの API 呼び出しは同一オリジンの `/api/backend` を通るため、公開 API の URL をビルドに焼き込む必要はありません。
- `web_server` の `/healthz` を liveness、DB 接続を確認する `/readyz` を readiness probe に使えます。`DATABASE_URL`、`ADMIN_USER_IDS`、大会期間は Secret/ConfigMap から注入してください。
- PostgreSQL は永続ボリューム付き StatefulSet またはマネージド DB に置き、`scripts/init.sql` は新規 DB の初期化時だけ実行します。14 系からの移行は `scripts/migrate-pg14-to-pg18.sh` を使います。
- `judge_server` は `HCCC_RUNNER_EXECUTABLE=/app/k8s-job-runner` で各提出を
  Kubernetes Job として実行します（イメージ内に同梱）。アダプタは `image`,
  `base64(asm)`, `exitcode` / `stdout` / `none`, `base64(JSON testcases)` を
  受け取り、test_runner と同じ終了コード・stderr を返します。runner イメージは
  `HCCC_RUNNER_IMAGE_X8664` / `HCCC_RUNNER_IMAGE_RISCV` で上書きできます。
- Pending 提出の取得は `SELECT ... FOR UPDATE SKIP LOCKED`＋リース
  （`claimed_at` / `claimed_by`、`JUDGE_CLAIM_LEASE_SECS` 既定 300 秒）で行うため、
  judge は複数レプリカで運用できます。詳細とマニフェストは `k8s/README.md` を参照してください。
