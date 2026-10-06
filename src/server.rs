use std::sync::Arc;
use std::time::Duration;

use axum::extract::State;
use axum::http::StatusCode;
use axum::response::sse::{Event, Sse};
use axum::response::{Html, IntoResponse};
use axum::routing::{get, post};
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use tokio_stream::wrappers::IntervalStream;
use tokio_stream::StreamExt;

use crate::config::{Config, Machine};
use crate::magic_packet::MagicPacket;
use crate::ping::{ping_host, Status};

#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
}

#[derive(Serialize)]
struct MachineStatus {
    name: String,
    mac: String,
    ip: Option<String>,
    status: Status,
}

#[derive(Deserialize)]
struct WakeRequest {
    name: String,
}

#[derive(Serialize)]
struct WakeResponse {
    success: bool,
    message: String,
}

pub async fn run_server(config: Arc<Config>) {
    let state = AppState {
        config: config.clone(),
    };

    let app = Router::new()
        .route("/", get(index_handler))
        .route("/api/machines", get(machines_handler))
        .route("/api/wake", post(wake_handler))
        .route("/api/status", get(status_sse_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(&config.listen).await.unwrap();
    println!("server listening on http://{}", config.listen);
    axum::serve(listener, app).await.unwrap();
}

async fn index_handler(State(state): State<AppState>) -> Html<String> {
    Html(render_index(&state.config))
}

async fn machines_handler(State(state): State<AppState>) -> Json<Vec<MachineStatus>> {
    let statuses = get_all_statuses(&state.config).await;
    Json(statuses)
}

async fn wake_handler(
    State(state): State<AppState>,
    Json(req): Json<WakeRequest>,
) -> impl IntoResponse {
    let machine = match state.config.find_machine(&req.name) {
        Some(m) => m,
        None => {
            return (
                StatusCode::NOT_FOUND,
                Json(WakeResponse {
                    success: false,
                    message: format!("machine '{}' not found", req.name),
                }),
            );
        }
    };

    let packet = match MagicPacket::from_str(&machine.mac) {
        Ok(p) => p,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(WakeResponse {
                    success: false,
                    message: format!("invalid MAC: {}", e),
                }),
            );
        }
    };

    let addr = state.config.resolve_broadcast(machine);
    match packet.broadcast(&addr) {
        Ok(()) => (
            StatusCode::OK,
            Json(WakeResponse {
                success: true,
                message: format!("magic packet sent to {} via {}", packet, addr),
            }),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(WakeResponse {
                success: false,
                message: format!("failed to send: {}", e),
            }),
        ),
    }
}

async fn status_sse_handler(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let interval = tokio::time::interval(Duration::from_secs(5));
    let stream = IntervalStream::new(interval).map(move |_| {
        let config = state.config.clone();
        let statuses: Vec<MachineStatus> = config
            .machines
            .iter()
            .map(|m| {
                let status = match &m.ip {
                    Some(ip) => ping_host(ip, Duration::from_secs(2)),
                    None => Status::Unknown,
                };
                MachineStatus {
                    name: m.name.clone(),
                    mac: m.mac.clone(),
                    ip: m.ip.clone(),
                    status,
                }
            })
            .collect();

        let data = serde_json::to_string(&statuses).unwrap_or_default();
        Ok(Event::default().data(data))
    });

    Sse::new(stream)
}

async fn get_all_statuses(config: &Config) -> Vec<MachineStatus> {
    config
        .machines
        .iter()
        .map(|m| {
            let status = match &m.ip {
                Some(ip) => ping_host(ip, Duration::from_secs(2)),
                None => Status::Unknown,
            };
            MachineStatus {
                name: m.name.clone(),
                mac: m.mac.clone(),
                ip: m.ip.clone(),
                status,
            }
        })
        .collect()
}

/// Renders a remote desktop link for machines reachable over the mesh.
///
/// The dashboard resolves the node from the overlay address, so nothing here
/// needs to know runetale's internal ids.
fn rdp_link(config: &Config, machine: &Machine) -> String {
    let (Some(dashboard), Some(overlay_ip)) = (&config.runetale_dashboard, &machine.runetale_ip)
    else {
        return String::new();
    };
    format!(
        r#" <a class="rdp" href="{base}/admin/rdp?ip={ip}&nodeName={name}" target="_blank" rel="noopener">RDP</a>"#,
        base = dashboard.trim_end_matches('/'),
        ip = urlencode(overlay_ip),
        name = urlencode(&machine.name),
    )
}

/// Percent-encodes the characters that can appear in a machine name or an
/// address. Both are operator-supplied, so they are not assumed to be safe.
fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{:02X}", byte)),
        }
    }
    out
}

fn render_index(config: &Config) -> String {
    let mut machines_html = String::new();
    for m in &config.machines {
        machines_html.push_str(&format!(
            r#"<tr id="row-{name}">
  <td>{name}</td>
  <td><code>{mac}</code></td>
  <td>{ip}</td>
  <td class="status" id="status-{name}">-</td>
  <td><button onclick="wake('{name}')">Wake</button>{rdp}</td>
</tr>"#,
            name = m.name,
            mac = m.mac,
            ip = m.ip.as_deref().unwrap_or("-"),
            rdp = rdp_link(config, m),
        ));
    }

    let mut schedules_html = String::new();
    for s in &config.schedules {
        schedules_html.push_str(&format!(
            "<tr><td>{}</td><td><code>{}</code></td></tr>",
            s.machine, s.cron
        ));
    }

    format!(
        r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>WoL - Wake-on-LAN</title>
<style>
  body {{ font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; max-width: 800px; margin: 2rem auto; padding: 0 1rem; }}
  table {{ width: 100%; border-collapse: collapse; margin: 1rem 0; }}
  th, td {{ text-align: left; padding: 0.5rem; border-bottom: 1px solid #eee; }}
  button {{ background: #2563eb; color: white; border: none; padding: 0.4rem 1rem; border-radius: 4px; cursor: pointer; }}
  button:hover {{ background: #1d4ed8; }}
  .rdp {{ display: inline-block; margin-left: 0.5rem; padding: 0.4rem 1rem; border-radius: 4px; background: #334155; color: white; text-decoration: none; font-size: 0.875rem; }}
  .rdp:hover {{ background: #1e293b; }}
  .online {{ color: #16a34a; font-weight: bold; }}
  .offline {{ color: #dc2626; }}
  .unknown {{ color: #9ca3af; }}
  .toast {{ position: fixed; top: 1rem; right: 1rem; padding: 0.75rem 1.5rem; border-radius: 6px; color: white; font-size: 0.9rem; display: none; }}
  .toast.success {{ background: #16a34a; }}
  .toast.error {{ background: #dc2626; }}
  h2 {{ margin-top: 2rem; }}
</style>
</head>
<body>
<h1>Wake-on-LAN</h1>

<h2>Machines</h2>
<table>
  <thead><tr><th>Name</th><th>MAC</th><th>IP</th><th>Status</th><th>Action</th></tr></thead>
  <tbody>{machines}</tbody>
</table>

{schedules_section}

<div id="toast" class="toast"></div>

<script>
function wake(name) {{
  fetch('/api/wake', {{
    method: 'POST',
    headers: {{'Content-Type': 'application/json'}},
    body: JSON.stringify({{name: name}})
  }})
  .then(r => r.json())
  .then(data => showToast(data.message, data.success ? 'success' : 'error'))
  .catch(e => showToast('Request failed: ' + e, 'error'));
}}

function showToast(msg, type) {{
  const t = document.getElementById('toast');
  t.textContent = msg;
  t.className = 'toast ' + type;
  t.style.display = 'block';
  setTimeout(() => t.style.display = 'none', 3000);
}}

const es = new EventSource('/api/status');
es.onmessage = function(e) {{
  const machines = JSON.parse(e.data);
  machines.forEach(m => {{
    const el = document.getElementById('status-' + m.name);
    if (el) {{
      el.textContent = m.status;
      el.className = 'status ' + m.status;
    }}
  }});
}};
</script>
</body>
</html>"#,
        machines = machines_html,
        schedules_section = if config.schedules.is_empty() {
            String::new()
        } else {
            format!(
                r#"<h2>Schedules</h2>
<table>
  <thead><tr><th>Machine</th><th>Cron</th></tr></thead>
  <tbody>{}</tbody>
</table>"#,
                schedules_html
            )
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn machine(name: &str, runetale_ip: Option<&str>) -> Machine {
        Machine {
            name: name.to_string(),
            mac: "aa:bb:cc:dd:ee:ff".to_string(),
            ip: Some("192.168.1.100".to_string()),
            broadcast: None,
            port: None,
            runetale_ip: runetale_ip.map(str::to_string),
        }
    }

    fn config(dashboard: Option<&str>) -> Config {
        Config {
            runetale_dashboard: dashboard.map(str::to_string),
            ..Config::default()
        }
    }

    #[test]
    fn links_when_both_dashboard_and_overlay_ip_are_set() {
        let html = rdp_link(
            &config(Some("https://console.example.com")),
            &machine("desktop", Some("100.107.0.36")),
        );
        assert!(html.contains("https://console.example.com/admin/rdp?ip=100.107.0.36"));
        assert!(html.contains("nodeName=desktop"));
    }

    #[test]
    fn renders_nothing_without_an_overlay_ip() {
        let html = rdp_link(
            &config(Some("https://console.example.com")),
            &machine("desktop", None),
        );
        assert!(html.is_empty());
    }

    #[test]
    fn renders_nothing_without_a_dashboard() {
        let html = rdp_link(&config(None), &machine("desktop", Some("100.107.0.36")));
        assert!(html.is_empty());
    }

    #[test]
    fn tolerates_a_trailing_slash_on_the_dashboard() {
        let html = rdp_link(
            &config(Some("https://console.example.com/")),
            &machine("desktop", Some("100.107.0.36")),
        );
        assert!(html.contains("https://console.example.com/admin/rdp"));
    }

    // Machine names come from the operator's config file, so they reach the
    // query string unsanitised unless encoded.
    #[test]
    fn encodes_characters_that_would_break_out_of_the_query_string() {
        let html = rdp_link(
            &config(Some("https://console.example.com")),
            &machine("my pc&x=1", Some("100.107.0.36")),
        );
        assert!(html.contains("nodeName=my%20pc%26x%3D1"));
        assert!(!html.contains("my pc&x=1"));
    }
}
