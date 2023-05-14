use nix::libc::{SIGCHLD, SIGINT, SIGTSTP};
use signal_hook::iterator::Signals;
use std::{sync::mpsc::Sender, thread};

use super::{shell_main::DynError, worker::WorkerMsg};

pub fn spawn_sig_handler(tx: Sender<WorkerMsg>) -> Result<(), DynError> {
    let mut signals = Signals::new(&[SIGINT, SIGTSTP, SIGCHLD])?;
    thread::spawn(move || {
        for sig in signals.forever() {
            tx.send(WorkerMsg::Signal(sig)).unwrap();
        }
    });
    Ok(())
}
