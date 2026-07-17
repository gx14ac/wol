mod config;
mod magic_packet;

use std::path::PathBuf;
use std::process;

use clap::{Parser, Subcommand};

use config::Config;
use magic_packet::MagicPacket;

#[derive(Parser)]
#[command(name = "wol", version, about = "Wake-on-LAN CLI tool")]
struct Cli {
    #[arg(short, long, help = "Path to config file")]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Send a magic packet to wake a device
    Send {
        #[arg(short, long, help = "MAC address (e.g. aa:bb:cc:dd:ee:ff)")]
        mac: Option<String>,

        #[arg(short, long, help = "Machine name from config")]
        name: Option<String>,

        #[arg(short, long, help = "Broadcast address")]
        broadcast: Option<String>,

        #[arg(short, long, help = "UDP port")]
        port: Option<u16>,
    },
    /// List configured machines
    List,
}

fn main() {
    let cli = Cli::parse();

    let config = load_config(&cli.config);

    match cli.command {
        Commands::Send {
            mac,
            name,
            broadcast,
            port,
        } => cmd_send(config, mac, name, broadcast, port),
        Commands::List => cmd_list(config),
    }
}

fn load_config(path: &Option<PathBuf>) -> Option<Config> {
    if let Some(p) = path {
        match Config::load(p) {
            Ok(c) => return Some(c),
            Err(e) => {
                eprintln!("error: failed to load config: {}", e);
                process::exit(1);
            }
        }
    }

    let candidates = [
        config_dir().map(|d| d.join("wol").join("config.yaml")),
        Some(PathBuf::from("config.yaml")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            match Config::load(&candidate) {
                Ok(c) => return Some(c),
                Err(e) => {
                    eprintln!("warning: failed to load {}: {}", candidate.display(), e);
                }
            }
        }
    }

    None
}

fn config_dir() -> Option<PathBuf> {
    #[cfg(target_os = "macos")]
    {
        std::env::var("HOME")
            .ok()
            .map(|h| PathBuf::from(h).join("Library").join("Application Support"))
    }
    #[cfg(target_os = "linux")]
    {
        std::env::var("XDG_CONFIG_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".config")))
    }
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(PathBuf::from)
    }
}

fn cmd_send(
    config: Option<Config>,
    mac: Option<String>,
    name: Option<String>,
    broadcast_override: Option<String>,
    port_override: Option<u16>,
) {
    if mac.is_none() && name.is_none() {
        eprintln!("error: either --mac or --name must be provided");
        process::exit(1);
    }
    if mac.is_some() && name.is_some() {
        eprintln!("error: --mac and --name are mutually exclusive");
        process::exit(1);
    }

    let (target_mac, resolved_addr) = if let Some(mac_str) = mac {
        let default_config = Config::default();
        let cfg = config.as_ref().unwrap_or(&default_config);
        let addr = match (&broadcast_override, port_override) {
            (Some(b), Some(p)) => format!("{}:{}", b, p),
            (Some(b), None) => format!("{}:{}", b, cfg.port),
            (None, Some(p)) => format!("{}:{}", cfg.broadcast, p),
            (None, None) => format!("{}:{}", cfg.broadcast, cfg.port),
        };
        (mac_str, addr)
    } else {
        let machine_name = name.unwrap();
        let cfg = match &config {
            Some(c) => c,
            None => {
                eprintln!("error: --name requires a config file");
                process::exit(1);
            }
        };
        let machine = match cfg.find_machine(&machine_name) {
            Some(m) => m,
            None => {
                eprintln!("error: machine '{}' not found in config", machine_name);
                process::exit(1);
            }
        };
        let mut addr = cfg.resolve_broadcast(machine);
        if let Some(b) = &broadcast_override {
            let port = port_override.unwrap_or(machine.port.unwrap_or(cfg.port));
            addr = format!("{}:{}", b, port);
        } else if let Some(p) = port_override {
            let broadcast = machine.broadcast.as_deref().unwrap_or(&cfg.broadcast);
            addr = format!("{}:{}", broadcast, p);
        }
        (machine.mac.clone(), addr)
    };

    let packet = match MagicPacket::from_str(&target_mac) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {}", e);
            process::exit(1);
        }
    };

    match packet.broadcast(&resolved_addr) {
        Ok(()) => {
            println!("magic packet sent to {} via {}", packet, resolved_addr);
        }
        Err(e) => {
            eprintln!("error: failed to send packet: {}", e);
            process::exit(1);
        }
    }
}

fn cmd_list(config: Option<Config>) {
    let cfg = match config {
        Some(c) => c,
        None => {
            eprintln!("error: no config file found");
            process::exit(1);
        }
    };

    if cfg.machines.is_empty() {
        println!("no machines configured");
        return;
    }

    println!("{:<15} {:<20} {}", "NAME", "MAC", "BROADCAST");
    for machine in &cfg.machines {
        let addr = cfg.resolve_broadcast(machine);
        println!("{:<15} {:<20} {}", machine.name, machine.mac, addr);
    }
}
