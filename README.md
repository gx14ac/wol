# wol

A Wake-on-LAN CLI tool written in Rust. Sends magic packets over UDP to wake devices on your network.

## Installation

Download a pre-built binary from [Releases](https://github.com/gx14ac/wol/releases), or build from source:

```bash
cargo install --path .
```

## Usage

### Send a magic packet by MAC address

```bash
wol send --mac aa:bb:cc:dd:ee:ff
```

### Specify broadcast address and port

```bash
wol send --mac aa:bb:cc:dd:ee:ff --broadcast 192.168.1.255 --port 9
```

### Send by machine name (requires config)

```bash
wol send --name desktop --config config.yaml
```

### List configured machines

```bash
wol list --config config.yaml
```

## Configuration

Create a `config.yaml` (see `config.example.yaml`):

```yaml
broadcast: "255.255.255.255"
port: 9

machines:
  - name: desktop
    mac: "aa:bb:cc:dd:ee:ff"
    ip: "192.168.1.100"
  - name: server
    mac: "11:22:33:44:55:66"
    broadcast: "10.0.0.255"
    port: 7
```

Config file search order (first found wins):
1. `--config` flag (explicit path)
2. `$XDG_CONFIG_HOME/wol/config.yaml` (Linux) / `~/Library/Application Support/wol/config.yaml` (macOS) / `%APPDATA%\wol\config.yaml` (Windows)
3. `./config.yaml` (current directory)

Per-machine `broadcast` and `port` override the global defaults.

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
