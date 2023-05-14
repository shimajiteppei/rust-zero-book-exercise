use nix::{
    libc::{self, SIGCHLD},
    unistd::{tcgetpgrp, Pid},
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    sync::mpsc::{Receiver, SyncSender},
    thread,
};

use super::{
    built_in_cmd::built_in_cmd,
    child_handler::{spawn_child, wait_child},
    shell_main::{DynError, ShellMsg},
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ProcState {
    Run,
    Stop,
}

#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub state: ProcState,
    pub pgid: Pid,
}

type CmdResult<'a> = Result<Vec<(&'a str, Vec<&'a str>)>, DynError>;

pub enum WorkerMsg {
    Signal(i32),
    Cmd(String),
}

#[derive(Debug)]
pub struct Worker {
    pub exit_val: i32,
    pub fg: Option<Pid>,
    pub jobs: BTreeMap<usize, (Pid, String)>,
    pub pgid_to_pids: HashMap<Pid, (usize, HashSet<Pid>)>,
    pub pid_to_info: HashMap<Pid, ProcInfo>,
    pub shell_pgid: Pid,
}

impl Worker {
    pub fn new() -> Self {
        Self {
            exit_val: 0,
            fg: None,
            jobs: BTreeMap::new(),
            pgid_to_pids: HashMap::new(),
            pid_to_info: HashMap::new(),
            shell_pgid: tcgetpgrp(libc::STDIN_FILENO).unwrap(),
        }
    }

    pub fn spawn(mut self, worker_tx: Receiver<WorkerMsg>, shell_tx: SyncSender<ShellMsg>) {
        thread::spawn(move || {
            for msg in worker_tx.iter() {
                match msg {
                    WorkerMsg::Cmd(line) => match parse_cmd(&line) {
                        Ok(cmd) => {
                            if built_in_cmd(&mut self, &cmd, &shell_tx) {
                                continue;
                            }

                            if !spawn_child(&mut self, &line, &cmd) {
                                shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
                            }
                        }
                        Err(e) => {
                            eprintln!("ZeroSh: {e}");
                            shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
                        }
                    },
                    WorkerMsg::Signal(SIGCHLD) => {
                        wait_child(&mut self, &shell_tx);
                    }
                    _ => (),
                }
            }
        });
    }
}

fn parse_cmd(line: &str) -> CmdResult {
    Ok(line
        .lines()
        .map(|l| {
            let mut words = l.split_ascii_whitespace();
            if let Some(cmd) = words.next() {
                (cmd, words.collect::<Vec<&str>>())
            } else {
                ("", Vec::new())
            }
        })
        .filter(|(cmd, _)| !cmd.is_empty())
        .collect::<Vec<(&str, Vec<&str>)>>())
}
