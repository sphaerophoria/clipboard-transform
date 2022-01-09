use clipboard_transforms::{ConfigMonitor, Mapping, Monitor, MonitorTray};

use directories::ProjectDirs;
use ksni::{Handle, TrayService};
use log::{error, info};
use structopt::StructOpt;

use std::{
    error::Error,
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};

#[derive(Debug, StructOpt)]
struct Opts {
    config: Option<PathBuf>,
}

impl Opts {
    fn config_path(&self) -> Result<PathBuf, String> {
        if let Some(c) = &self.config {
            return Ok(c.clone());
        }

        if let Some(dirs) = ProjectDirs::from("com", "micksayson", "clipboard-transforms") {
            let dir = dirs.config_dir().to_owned();
            if !dir.exists() {
                std::fs::create_dir_all(&dir)
                    .map_err(|_| "Failed to create config directory".to_string())?;
            }

            let config_file = dir.join("config.json");
            if !config_file.exists() {
                let mut f = std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .open(config_file)
                    .map_err(|_| "Failed to create default config".to_string())?;

                f.write(
                    "[ \n\
                    {\n\
                        \t\"type\": \"simple\",\n\
                        \t\"left\": \"left\",\n\
                        \t\"right\": \"right\"\n\
                    }\n\
                ]"
                    .as_bytes(),
                )
                .map_err(|_| "Failed to write default config".to_string())?;
            }

            return Ok(dir.join("config.json"));
        }

        Err("Failed to find config".into())
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();
    let opts = Opts::from_args();

    let config_path = opts.config_path()?;
    let mappings = Arc::new(Mutex::new(clipboard_transforms::load_mappings(
        &config_path,
    )?));
    info!("{:?}", mappings);

    let config = ConfigMonitor::new(config_path.clone())?;

    let monitor_tray = MonitorTray::new(Arc::clone(&mappings), config_path);
    let service = TrayService::new(monitor_tray);
    let tray_handle = service.handle();
    service.spawn();

    let thread_mappings = Arc::clone(&mappings);
    std::thread::spawn(move || config_watch_thread(config, thread_mappings, tray_handle));

    let monitor = Monitor::new()?;
    monitor.run(&mappings);

    Ok(())
}

fn config_watch_thread(
    mut config_monitor: ConfigMonitor,
    mappings: Arc<Mutex<Vec<Mapping>>>,
    tray_handle: Handle<MonitorTray>,
) {
    loop {
        let new_mappings = match config_monitor.recv() {
            Ok(x) => x,
            Err(e) => {
                error!("{}", e);
                continue;
            }
        };

        info!("Loaded new mappings {:?}", new_mappings);
        let new_mappings = new_mappings;
        *mappings.lock().expect("Poisoned mappings lock") = new_mappings;
        // Force a UI update
        tray_handle.update(|_| {});
    }
}
