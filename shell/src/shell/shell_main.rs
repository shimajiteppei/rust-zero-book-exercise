use nix::sys::signal::{signal, SigHandler, Signal};
use rustyline::{error::ReadlineError, history::FileHistory, Editor};
use std::{
    process::exit,
    sync::mpsc::{channel, sync_channel},
};

use super::{
    signal_handler::spawn_sig_handler,
    worker::{Worker, WorkerMsg},
};

pub type DynError = Box<dyn std::error::Error + 'static>;

pub enum ShellMsg {
    Continue(i32),
    Quit(i32),
}

#[derive(Debug)]
pub struct Shell {
    logfile: String,
}

impl Shell {
    pub fn new(logfile: &str) -> Self {
        Self {
            logfile: logfile.to_string(),
        }
    }

    pub fn run(&self) -> Result<(), DynError> {
        unsafe { signal(Signal::SIGTTOU, SigHandler::SigIgn).unwrap() };

        let mut rl = Editor::<(), FileHistory>::new()?;
        if let Err(e) = rl.load_history(&self.logfile) {
            eprintln!("ZeroSh: ヒストリファイルの読み込みに失敗: {e}");
        }

        let (worker_tx, worker_rx) = channel();
        let (shell_tx, shell_rx) = sync_channel(0);
        spawn_sig_handler(worker_tx.clone())?;
        Worker::new().spawn(worker_rx, shell_tx);

        let exit_val;
        let mut prev = 0;
        loop {
            let face = if prev == 0 { '🙂' } else { '💀' };
            match rl.readline(&format!("ZeroSh {face} %> ")) {
                Ok(line) => {
                    let line_trimmed = line.trim();
                    if line_trimmed.is_empty() {
                        continue;
                    } else {
                        rl.add_history_entry(line_trimmed)?;
                    }

                    worker_tx.send(WorkerMsg::Cmd(line)).unwrap();
                    match shell_rx.recv().unwrap() {
                        ShellMsg::Continue(n) => prev = n,
                        ShellMsg::Quit(n) => {
                            exit_val = n;
                            break;
                        }
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    eprintln!("ZeroSh: 終了はCtrl+d");
                }
                Err(ReadlineError::Eof) => {
                    worker_tx.send(WorkerMsg::Cmd("exit".to_string())).unwrap();
                    match shell_rx.recv().unwrap() {
                        ShellMsg::Quit(n) => {
                            exit_val = n;
                            break;
                        }
                        _ => panic!("exitに失敗"),
                    }
                }
                Err(e) => {
                    eprintln!("ZeroSh: 読み込みエラー\n{e}");
                    exit_val = 1;
                    break;
                }
            }
        }

        if let Err(e) = rl.save_history(&self.logfile) {
            eprintln!("ZeroSh: ヒストリファイルへの書き込みに失敗: {e}");
        }
        exit(exit_val);
    }
}
