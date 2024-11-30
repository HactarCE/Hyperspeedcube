use parking_lot::Mutex;
use std::sync::mpsc;

#[cfg_attr(not(target_arch = "wasm32"), path = "local.rs")]
#[cfg_attr(target_arch = "wasm32", path = "web.rs")]
mod platform;

pub use platform::*;

use crate::PrefsConvert;

lazy_static! {
    pub(crate) static ref PREFS_SAVE_THREAD: (
        mpsc::Sender<PrefsSaveCommand>,
        Mutex<Option<std::thread::JoinHandle<()>>>
    ) = spawn_save_thread();
}

fn spawn_save_thread() -> (
    mpsc::Sender<PrefsSaveCommand>,
    Mutex<Option<std::thread::JoinHandle<()>>>,
) {
    let (tx, rx) = mpsc::channel();

    let join_handle = std::thread::spawn(move || {
        for command in rx {
            match command {
                PrefsSaveCommand::Save(prefs) => {
                    let result = platform::save(&prefs.to_serde());
                    match result {
                        Ok(()) => log::debug!("Saved preferences"),
                        Err(e) => log::error!("Error saving preferences: {e}"),
                    }
                }
                PrefsSaveCommand::Quit => return,
            }
        }
    });

    (tx, Mutex::new(Some(join_handle)))
}

#[derive(Debug)]
pub(crate) enum PrefsSaveCommand {
    Save(crate::schema::current::Preferences),
    Quit,
}
