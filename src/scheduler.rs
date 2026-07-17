use std::sync::Arc;

use chrono::Utc;
use cron::Schedule as CronSchedule;
use tokio::time::{interval, Duration};

use crate::config::Config;
use crate::magic_packet::MagicPacket;

pub struct Scheduler {
    config: Arc<Config>,
}

impl Scheduler {
    pub fn new(config: Arc<Config>) -> Self {
        Self { config }
    }

    pub fn validate(&self) -> Result<(), String> {
        for schedule in &self.config.schedules {
            if self.config.find_machine(&schedule.machine).is_none() {
                return Err(format!(
                    "schedule references unknown machine: '{}'",
                    schedule.machine
                ));
            }
            schedule.cron.parse::<CronSchedule>().map_err(|e| {
                format!(
                    "invalid cron expression '{}' for machine '{}': {}",
                    schedule.cron, schedule.machine, e
                )
            })?;
        }
        Ok(())
    }

    pub async fn run(self) {
        if self.config.schedules.is_empty() {
            return;
        }

        let mut ticker = interval(Duration::from_secs(30));

        loop {
            ticker.tick().await;
            let now = Utc::now();

            for schedule in &self.config.schedules {
                let cron: CronSchedule = match schedule.cron.parse() {
                    Ok(c) => c,
                    Err(_) => continue,
                };

                if let Some(next) = cron.upcoming(Utc).next() {
                    let diff = (next - now).num_seconds();
                    if diff <= 30 && diff >= 0 {
                        self.wake_machine(&schedule.machine);
                    }
                }
            }
        }
    }

    fn wake_machine(&self, name: &str) {
        let machine = match self.config.find_machine(name) {
            Some(m) => m,
            None => return,
        };

        let packet = match MagicPacket::from_str(&machine.mac) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[scheduler] invalid MAC for '{}': {}", name, e);
                return;
            }
        };

        let addr = self.config.resolve_broadcast(machine);
        match packet.broadcast(&addr) {
            Ok(()) => println!("[scheduler] woke '{}' ({}) via {}", name, packet, addr),
            Err(e) => eprintln!("[scheduler] failed to wake '{}': {}", name, e),
        }
    }
}
