use chrono::{DateTime, TimeDelta, Utc};
use std::cmp::max;
use tracing::debug;

pub struct Tracker {
    last_input_time: DateTime<Utc>,
    is_idle: bool,
    is_changed: bool,
    idle_timeout: TimeDelta,

    idle_end: Option<DateTime<Utc>>,
}

#[derive(Debug)]
pub enum Status {
    Idle {
        changed: bool,
        last_input_time: DateTime<Utc>,
        duration: TimeDelta,
    },
    Active {
        changed: bool,
        last_input_time: DateTime<Utc>,
    },
}

impl Tracker {
    pub fn new(now: DateTime<Utc>, idle_timeout: TimeDelta) -> Self {
        Self {
            last_input_time: now,
            is_idle: false,
            is_changed: false,
            idle_timeout,
            idle_end: None,
        }
    }

    fn set_idle(&mut self, is_idle: bool) {
        self.is_idle = is_idle;
        self.is_changed = true;
    }

    pub fn mark_not_idle(&mut self, now: DateTime<Utc>) {
        debug!("No longer idle");
        self.last_input_time = now;
        self.set_idle(false);

        self.idle_end = Some(now);
    }

    pub fn mark_idle(&mut self, _: DateTime<Utc>) {
        debug!("Idle again");
        self.set_idle(true);
    }

    // The logic is rewritten from the original Python code:
    // https://github.com/ActivityWatch/aw-watcher-afk/blob/ef531605cd8238e00138bbb980e5457054e05248/aw_watcher_afk/afk.py#L73
    pub fn get_with_last_input(
        &mut self,
        now: DateTime<Utc>,
        seconds_since_input: u32,
    ) -> anyhow::Result<Status> {
        let time_since_input = TimeDelta::seconds(i64::from(seconds_since_input));

        self.last_input_time = now - time_since_input;

        if self.is_idle
            && u64::from(seconds_since_input) < self.idle_timeout.num_seconds().try_into().unwrap()
        {
            debug!("No longer idle");
            self.set_idle(false);
        } else if !self.is_idle
            && u64::from(seconds_since_input) >= self.idle_timeout.num_seconds().try_into().unwrap()
        {
            debug!("Idle again");
            self.set_idle(true);
        }

        Ok(self.get_status(now))
    }

    pub fn get_reactive(&mut self, now: DateTime<Utc>) -> anyhow::Result<Status> {
        if !self.is_idle {
            self.last_input_time = max(self.last_input_time, now - self.idle_timeout);

            if let Some(idle_end) = self.idle_end {
                if self.last_input_time < idle_end {
                    self.last_input_time = idle_end;
                }
            }
        }

        Ok(self.get_status(now))
    }

    fn get_status(&mut self, now: DateTime<Utc>) -> Status {
        let result = if self.is_changed {
            if self.is_idle {
                Status::Idle {
                    changed: self.is_changed,
                    last_input_time: self.last_input_time,
                    duration: now - self.last_input_time,
                }
            } else {
                Status::Active {
                    changed: self.is_changed,
                    last_input_time: self.last_input_time,
                }
            }
        } else if self.is_idle {
            Status::Idle {
                changed: self.is_changed,
                last_input_time: self.last_input_time,
                duration: now - self.last_input_time,
            }
        } else {
            Status::Active {
                changed: self.is_changed,
                last_input_time: self.last_input_time,
            }
        };
        self.is_changed = false;

        result
    }
}
