// SPDX-License-Identifier: EUPL-1.2

use std::io::{IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

#[derive(Clone)]
pub enum PinStatus {
    Pending,
    Fetching,
    NoChange,
    Updated { old: String, new: String },
    Drift { rev: String, accepted: bool },
    Failed(String),
}

const FRAMES: [char; 4] = ['/', '-', '\\', '|'];

pub struct Display {
    states: Arc<Mutex<Vec<PinStatus>>>,
    names: Arc<Vec<String>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
    tty: bool,
}

impl Display {
    pub fn new(names: Vec<String>) -> Self {
        let tty = std::io::stdout().is_terminal();
        let states = Arc::new(Mutex::new(vec![PinStatus::Pending; names.len()]));
        let names = Arc::new(names);
        let stop = Arc::new(AtomicBool::new(false));

        let handle = tty.then(|| {
            let (states, names, stop) = (states.clone(), names.clone(), stop.clone());
            std::thread::spawn(move || {
                let mut drawn = false;
                let mut frame = 0;
                while !stop.load(Ordering::Relaxed) {
                    draw(&names, &states.lock().unwrap(), frame, drawn);
                    drawn = true;
                    frame = frame.wrapping_add(1);
                    std::thread::sleep(Duration::from_millis(80));
                }
                draw(&names, &states.lock().unwrap(), frame, drawn);
            })
        });
        Display {
            states,
            names,
            stop,
            handle,
            tty,
        }
    }

    pub fn set(&self, i: usize, status: PinStatus) {
        self.states.lock().unwrap()[i] = status;
    }

    pub fn finish(mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
        if !self.tty {
            let states = self.states.lock().unwrap();
            for (name, st) in self.names.iter().zip(states.iter()) {
                if let Some(line) = plain_line(name, st) {
                    println!("{line}");
                }
            }
        }
    }
}

fn draw(names: &[String], states: &[PinStatus], frame: usize, drawn: bool) {
    let mut out = String::new();
    if drawn {
        out.push_str(&format!("\x1b[{}A", names.len()));
    }
    for (name, st) in names.iter().zip(states) {
        out.push_str("\x1b[2K");
        out.push_str(&format!("[{}] {name}{}\n", glyph(st, frame), suffix(st)));
    }
    let mut so = std::io::stdout().lock();
    let _ = so.write_all(out.as_bytes());
    let _ = so.flush();
}

fn glyph(st: &PinStatus, frame: usize) -> String {
    let (color, ch) = match st {
        PinStatus::Pending => (2, '·'),
        PinStatus::Fetching => (34, FRAMES[frame % FRAMES.len()]),
        PinStatus::NoChange => (33, '-'),
        PinStatus::Updated { .. } => (32, '✓'),
        PinStatus::Drift { accepted: true, .. } => (33, '~'),
        PinStatus::Drift {
            accepted: false, ..
        } => (31, '!'),
        PinStatus::Failed(_) => (31, '✗'),
    };
    format!("\x1b[{color}m{ch}\x1b[0m")
}

fn suffix(st: &PinStatus) -> String {
    match st {
        PinStatus::Updated { old, new } => format!("  {old} -> {new}"),
        PinStatus::Drift {
            rev,
            accepted: false,
        } => {
            format!("  DRIFT: rev {rev} unchanged but content differs (lock kept)")
        }
        PinStatus::Drift {
            rev,
            accepted: true,
        } => {
            format!("  DRIFT: rev {rev} content changed, relocked (--accept)")
        }
        PinStatus::Failed(msg) => format!("  {msg}"),
        _ => String::new(),
    }
}

fn plain_line(name: &str, st: &PinStatus) -> Option<String> {
    match st {
        PinStatus::Updated { old, new } => Some(format!("{name}: {old} -> {new}")),
        PinStatus::NoChange => Some(format!("{name}: unchanged")),
        PinStatus::Drift {
            rev,
            accepted: false,
        } => Some(format!(
            "{name}: DRIFT: rev {rev} unchanged but content differs (lock kept)"
        )),
        PinStatus::Drift {
            rev,
            accepted: true,
        } => Some(format!(
            "{name}: DRIFT: rev {rev} content changed, relocked (--accept)"
        )),
        PinStatus::Failed(msg) => Some(format!("{name}: FAILED: {msg}")),
        _ => None,
    }
}
