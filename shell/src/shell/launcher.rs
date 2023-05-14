use super::shell_main::{DynError, Shell};

const HISTORY_FILE: &str = ".zerosh_history";

pub fn launch_shell() -> Result<(), DynError> {
    let mut logfile = HISTORY_FILE;
    let mut home = dirs::home_dir();
    if let Some(h) = &mut home {
        h.push(HISTORY_FILE);
        logfile = h.to_str().unwrap_or(HISTORY_FILE);
    }

    let sh = Shell::new(logfile);
    sh.run()?;

    Ok(())
}
