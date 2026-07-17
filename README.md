# wol

A Wake-on-LAN CLI tool written in Rust. Sends magic packets over UDP to wake devices on your network, with a built-in web UI and scheduled wake-ups.

**[日本語ドキュメント](docs/usage_ja.md)**

## Installation

Download a pre-built binary from [Releases](https://github.com/gx14ac/wol/releases), or build from source:

```bash
cargo install --path .
```

## Commands

| Command | Description |
|---------|-------------|
| `wol send` | Send a magic packet to wake a device |
| `wol list` | List configured machines |
| `wol serve` | Start web UI with status monitoring and scheduled wake-ups |

## Usage

### Send a magic packet

```bash
wol send --mac aa:bb:cc:dd:ee:ff
wol send --mac aa:bb:cc:dd:ee:ff --broadcast 192.168.1.255 --port 9
wol --config config.yaml send --name desktop
```

### List configured machines

```bash
wol --config config.yaml list
```

### Start web server

```bash
wol --config config.yaml serve
```

Opens a web UI at `http://localhost:7777` with:
- Real-time machine status (online/offline via ICMP ping, updated every 5s)
- One-click wake button per machine
- Cron-based scheduled wake-ups

## Configuration

Create a `config.yaml` (see `config.example.yaml`):

```yaml
listen: "0.0.0.0:7777"
broadcast: "255.255.255.255"
port: 9

machines:
  - name: desktop
    mac: "aa:bb:cc:dd:ee:ff"
    ip: "192.168.1.100"
  - name: server
    mac: "11:22:33:44:55:66"
    ip: "192.168.1.200"
    broadcast: "10.0.0.255"
    port: 7

schedules:
  - machine: desktop
    cron: "0 30 8 * * Mon-Fri"
```

Config file search order (first found wins):
1. `--config` flag (explicit path)
2. `$XDG_CONFIG_HOME/wol/config.yaml` (Linux) / `~/Library/Application Support/wol/config.yaml` (macOS) / `%APPDATA%\wol\config.yaml` (Windows)
3. `./config.yaml` (current directory)

### API endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/` | Web UI |
| GET | `/api/machines` | Machine statuses as JSON |
| POST | `/api/wake` | Send magic packet (`{"name": "desktop"}`) |
| GET | `/api/status` | SSE stream of machine statuses |

## How it works

WoL sends a "magic packet" — 6 bytes of `0xFF` followed by the target MAC address repeated 16 times (102 bytes total) — via UDP broadcast. Any NIC on the LAN with WoL enabled and matching MAC will power on the machine.

### Requirements on the target machine

- NIC Wake-on-LAN enabled (Linux: `ethtool -s eth0 wol g`)
- BIOS/UEFI WoL setting enabled
- Machine must be on the same broadcast domain (or use a subnet-directed broadcast address)

## Supported platforms

- Linux (x86_64, aarch64)
- macOS (x86_64, aarch64)
- Windows (x86_64)

## License

MIT
