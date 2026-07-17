use std::net::IpAddr;
use std::process::Command;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Online,
    Offline,
    Unknown,
}

pub fn ping_host(ip: &str, timeout: Duration) -> Status {
    let addr: IpAddr = match ip.parse() {
        Ok(a) => a,
        Err(_) => return Status::Unknown,
    };

    let timeout_secs = timeout.as_secs().max(1).to_string();

    let result = if cfg!(target_os = "windows") {
        Command::new("ping")
            .args(["-n", "1", "-w", &(timeout.as_millis().to_string()), &addr.to_string()])
            .output()
    } else {
        Command::new("ping")
            .args(["-c", "1", "-W", &timeout_secs, &addr.to_string()])
            .output()
    };

    match result {
        Ok(output) => {
            if output.status.success() {
                Status::Online
            } else {
                Status::Offline
            }
        }
        Err(_) => Status::Unknown,
    }
}
