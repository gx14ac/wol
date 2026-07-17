# WoL CLI 使い方ガイド

Wake-on-LAN (WoL) マジックパケットを送信してリモートマシンを起動するCLIツール。

## インストール

[Releases](https://github.com/gx14ac/wol/releases) からバイナリをダウンロード。

```bash
# macOS (Apple Silicon)
curl -L https://github.com/gx14ac/wol/releases/download/v0.2.0/wol-aarch64-apple-darwin.tar.gz | tar xz
sudo mv wol /usr/local/bin/

# macOS (Intel)
curl -L https://github.com/gx14ac/wol/releases/download/v0.2.0/wol-x86_64-apple-darwin.tar.gz | tar xz
sudo mv wol /usr/local/bin/

# Linux (x86_64)
curl -L https://github.com/gx14ac/wol/releases/download/v0.2.0/wol-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv wol /usr/local/bin/
```

## コマンド一覧

| コマンド | 説明 |
|----------|------|
| `wol send` | マジックパケットを送信して対象マシンを起動 |
| `wol list` | 設定ファイルに登録されたマシン一覧を表示 |
| `wol serve` | Web UI + ステータス監視 + スケジュール実行サーバーを起動 |

## 基本的な使い方

### MACアドレスを直接指定して送信

```bash
wol send --mac aa:bb:cc:dd:ee:ff
```

### ブロードキャストアドレスとポートを指定

```bash
wol send --mac aa:bb:cc:dd:ee:ff --broadcast 192.168.1.255 --port 9
```

### 設定ファイルからマシン名で送信

```bash
wol --config config.yaml send --name desktop
```

### 登録マシン一覧

```bash
wol --config config.yaml list
```

## 設定ファイル

`config.yaml` を作成:

```yaml
# サーバーモード時のリッスンアドレス
listen: "0.0.0.0:7777"

# デフォルトのブロードキャストアドレスとポート
broadcast: "192.168.1.255"
port: 9

# マシン定義
machines:
  - name: desktop
    mac: "aa:bb:cc:dd:ee:ff"
    ip: "192.168.1.100"          # (任意) ステータス監視用のIPアドレス
  - name: server
    mac: "11:22:33:44:55:66"
    ip: "192.168.1.200"
    broadcast: "10.0.0.255"      # (任意) マシン固有のブロードキャスト
    port: 7                      # (任意) マシン固有のポート

# スケジュール (cron式)
schedules:
  - machine: desktop
    cron: "0 30 8 * * Mon-Fri"   # 平日 08:30 に起動
  - machine: server
    cron: "0 0 6 * * *"          # 毎日 06:00 に起動
```

### 設定ファイルの探索順序

`--config` を指定しない場合、以下の順で探索:

1. `~/Library/Application Support/wol/config.yaml` (macOS)
2. `~/.config/wol/config.yaml` (Linux)
3. `%APPDATA%\wol\config.yaml` (Windows)
4. `./config.yaml` (カレントディレクトリ)

## Web UI (serve モード)

```bash
wol --config config.yaml serve
```

ブラウザで `http://localhost:7777` を開く。

### 機能

- マシン一覧とリアルタイムのオンライン/オフライン状態表示 (5秒間隔)
- ワンクリックでWoLパケット送信
- 設定されたスケジュール一覧の表示
- スケジュールに基づく自動起動

### API エンドポイント

| メソッド | パス | 説明 |
|----------|------|------|
| GET | `/` | Web UI |
| GET | `/api/machines` | 全マシンのステータスをJSON取得 |
| POST | `/api/wake` | マジックパケット送信 (`{"name": "desktop"}`) |
| GET | `/api/status` | SSE (Server-Sent Events) でリアルタイムステータス配信 |

### API 使用例

```bash
# マシン状態取得
curl http://localhost:7777/api/machines

# 起動指示
curl -X POST http://localhost:7777/api/wake \
  -H 'Content-Type: application/json' \
  -d '{"name": "desktop"}'
```

## スケジュール設定

cron式は6フィールド (秒 分 時 日 月 曜日):

```
秒(0-59) 分(0-59) 時(0-23) 日(1-31) 月(1-12) 曜日(Mon-Sun)
```

例:
- `0 30 8 * * Mon-Fri` — 平日 08:30
- `0 0 6 * * *` — 毎日 06:00
- `0 0 */2 * * *` — 2時間ごと

## 対象マシン側の設定

WoLが動作するには、対象マシン側で以下の設定が必要:

### Linux

```bash
# WoL対応確認
sudo ethtool eth0 | grep Wake-on

# WoL有効化 (g = magic packet)
sudo ethtool -s eth0 wol g

# 永続化 (systemd-networkd)
# /etc/systemd/network/eth0.network に追加:
# [Link]
# WakeOnLan=magic
```

### Windows

1. デバイスマネージャー → ネットワークアダプター → プロパティ
2. 「電源の管理」タブ → 「このデバイスでコンピューターのスタンバイ状態を解除できるようにする」を有効化
3. 「詳細設定」タブ → 「Wake on Magic Packet」を有効化

### macOS

システム設定 → 省エネルギー → 「ネットワークアクセスによるスリープ解除」を有効化

### 共通要件

- BIOS/UEFI で WoL が有効になっていること
- 送信元と対象マシンが同一ブロードキャストドメインにあること
  (異なるサブネットの場合はサブネットディレクテッドブロードキャストアドレスを使用)

## トラブルシューティング

### パケットが届かない

```bash
# 送信側でパケットが出ているか確認 (別ターミナルで実行)
sudo tcpdump -i any udp port 9

# 対象のMACアドレスが正しいか確認
ip link show       # Linux
ifconfig           # macOS
ipconfig /all      # Windows
```

### マシンが起動しない

1. WoLが有効か確認 (上記「対象マシン側の設定」参照)
2. ブロードキャストアドレスが正しいか確認 (同一サブネット)
3. ファイアウォールがUDP port 9をブロックしていないか確認
