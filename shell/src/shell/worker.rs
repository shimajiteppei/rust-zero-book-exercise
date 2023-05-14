use nix::{
    libc::{self, SIGCHLD},
    sys::{
        signal::{killpg, Signal},
        wait::{waitpid, WaitPidFlag, WaitStatus},
    },
    unistd::{self, dup2, execvp, fork, pipe, setpgid, tcgetpgrp, tcsetpgrp, ForkResult, Pid},
};
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    ffi::CString,
    mem::replace,
    process::exit,
    sync::mpsc::{Receiver, SyncSender},
    thread,
};

use super::shell_main::{DynError, ShellMsg};

fn syscall<F, T>(f: F) -> Result<T, nix::Error>
where
    F: Fn() -> Result<T, nix::Error>,
{
    loop {
        match f() {
            Err(nix::Error::EINTR) => (),
            result => return result,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
enum ProcState {
    Run,
    Stop,
}

#[derive(Debug, Clone)]
struct ProcInfo {
    state: ProcState,
    pgid: Pid,
}

type CmdResult<'a> = Result<Vec<(&'a str, Vec<&'a str>)>, DynError>;

struct CleanUp<F>
where
    F: Fn(),
{
    f: F,
}

impl<F> Drop for CleanUp<F>
where
    F: Fn(),
{
    fn drop(&mut self) {
        (self.f)()
    }
}

pub enum WorkerMsg {
    Signal(i32),
    Cmd(String),
}

#[derive(Debug)]
pub struct Worker {
    exit_val: i32,
    fg: Option<Pid>,
    jobs: BTreeMap<usize, (Pid, String)>,
    pgid_to_pids: HashMap<Pid, (usize, HashSet<Pid>)>,
    pid_to_info: HashMap<Pid, ProcInfo>,
    shell_pgid: Pid,
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
                            if self.built_in_cmd(&cmd, &shell_tx) {
                                continue;
                            }

                            if !self.spawn_child(&line, &cmd) {
                                shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
                            }
                        }
                        Err(e) => {
                            eprintln!("ZeroSh: {e}");
                            shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
                        }
                    },
                    WorkerMsg::Signal(SIGCHLD) => {
                        self.wait_child(&shell_tx);
                    }
                    _ => (),
                }
            }
        });
    }

    fn built_in_cmd(&mut self, cmd: &[(&str, Vec<&str>)], shell_tx: &SyncSender<ShellMsg>) -> bool {
        if cmd.len() > 1 {
            return false;
        }

        match cmd[0].0 {
            "exit" => self.run_exit(&cmd[0].1, shell_tx),
            "jobs" => todo!(),
            "fg" => self.run_fg(&cmd[0].1, shell_tx),
            "cd" => todo!(),
            _ => false,
        }
    }

    fn run_exit(&mut self, args: &[&str], shell_tx: &SyncSender<ShellMsg>) -> bool {
        if !self.jobs.is_empty() {
            eprintln!("ジョブが実行中なので終了できません");
            self.exit_val = 1;
            shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
            return true;
        }

        let exit_val = match args.get(1) {
            Some(s) => match (*s).parse::<i32>() {
                Ok(n) => n,
                _ => {
                    eprintln!("{s}は不正な引数です");
                    self.exit_val = 1;
                    shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
                    return true;
                }
            },
            None => self.exit_val,
        };

        shell_tx.send(ShellMsg::Quit(exit_val)).unwrap();
        true
    }

    fn run_fg(&mut self, args: &[&str], shell_tx: &SyncSender<ShellMsg>) -> bool {
        self.exit_val = 1;

        if args.len() < 2 {
            eprintln!("usage: fg 数字");
            shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
            return true;
        }

        if let Ok(n) = args[1].parse::<usize>() {
            if let Some((pgid, cmd)) = self.jobs.get(&n) {
                eprintln!("[{n}] 再開\t{cmd}");

                self.fg = Some(*pgid);
                tcsetpgrp(libc::STDIN_FILENO, *pgid).unwrap();

                killpg(*pgid, Signal::SIGCONT).unwrap();
                return true;
            }
        }

        eprintln!("{}というジョブは見つかりませんでした", args[1]);
        shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
        true
    }

    fn spawn_child(&mut self, line: &str, cmd: &[(&str, Vec<&str>)]) -> bool {
        assert_ne!(cmd.len(), 0);

        let job_id = match self.get_new_job_id() {
            Some(id) => id,
            None => {
                eprintln!("ZeroSh: 管理可能なジョブの最大値に到達");
                return false;
            }
        };

        if cmd.len() > 2 {
            eprintln!("ZeroSh: 3つ以上のコマンドによるパイプはサポートしていません");
            return false;
        }

        let mut input = None;
        let mut output = None;
        if cmd.len() == 2 {
            let p = pipe().unwrap();
            input = Some(p.0);
            output = Some(p.1);
        };

        let cleanup_pipe = CleanUp {
            f: || {
                if let Some(fd) = input {
                    syscall(|| unistd::close(fd)).unwrap()
                }
                if let Some(fd) = output {
                    syscall(|| unistd::close(fd)).unwrap()
                }
            },
        };

        let pgid;
        match fork_exec(Pid::from_raw(0), cmd[0].0, &cmd[0].1, None, output) {
            Ok(child) => {
                pgid = child;
            }
            Err(e) => {
                eprintln!("ZeroSh: プロセス生成エラー: {e}");
                return false;
            }
        };

        let info = ProcInfo {
            state: ProcState::Run,
            pgid,
        };
        let mut pids = HashMap::new();
        pids.insert(pgid, info.clone());

        if cmd.len() == 2 {
            match fork_exec(pgid, cmd[1].0, &cmd[1].1, input, None) {
                Ok(child) => {
                    pids.insert(child, info);
                }
                Err(e) => {
                    eprintln!("ZeroSh: プロセス生成エラー: {e}");
                    return false;
                }
            }
        };

        std::mem::drop(cleanup_pipe);

        self.fg = Some(pgid);
        self.insert_job(job_id, pgid, pids, line);
        tcsetpgrp(libc::STDIN_FILENO, pgid).unwrap();

        true
    }

    fn wait_child(&mut self, shell_tx: &SyncSender<ShellMsg>) {
        let flag = Some(WaitPidFlag::WUNTRACED | WaitPidFlag::WNOHANG | WaitPidFlag::WCONTINUED);

        loop {
            match syscall(|| waitpid(Pid::from_raw(-1), flag)) {
                Ok(WaitStatus::Exited(pid, status)) => {
                    self.exit_val = status;
                    self.process_term(pid, shell_tx);
                }
                Ok(WaitStatus::Signaled(pid, sig, core)) => {
                    eprintln!(
                        "\nZeroSh: 子プロセスがシグナルにより終了{}: pid = {pid}, signal = {sig}",
                        if core { " (コアダンプ) " } else { "" }
                    );
                    self.exit_val = sig as i32 + 128;
                    self.process_term(pid, shell_tx);
                }
                Ok(WaitStatus::Stopped(pid, _sig)) => {
                    self.process_stop(pid, shell_tx);
                }
                Ok(WaitStatus::Continued(pid)) => {
                    self.process_continue(pid);
                }
                Ok(WaitStatus::StillAlive) => {
                    return;
                }
                Err(nix::Error::ECHILD) => {
                    return;
                }
                Err(e) => {
                    eprintln!("\nZeroSh: waitが失敗: {e}");
                    exit(1);
                }
                #[cfg(any(target_os = "linux", target_os = "android"))]
                Ok(WaitStatus::PtraceEvent(pid, _, _) | WaitStatus::PtraceSyscall(pid)) => {
                    self.process_stop(pid, shell_tx)
                }
            }
        }
    }

    fn process_term(&mut self, pid: Pid, shell_tx: &SyncSender<ShellMsg>) {
        if let Some((job_id, pgid)) = self.remove_pid(pid) {
            self.manage_job(job_id, pgid, shell_tx);
        }
    }

    fn process_stop(&mut self, pid: Pid, shell_tx: &SyncSender<ShellMsg>) {
        self.set_pid_state(pid, ProcState::Stop);
        let pgid = self.pid_to_info.get(&pid).unwrap().pgid;
        let job_id = self.pgid_to_pids.get(&pgid).unwrap().0;
        self.manage_job(job_id, pgid, shell_tx);
    }

    fn process_continue(&mut self, pid: Pid) {
        self.set_pid_state(pid, ProcState::Run);
    }

    fn manage_job(&mut self, job_id: usize, pgid: Pid, shell_tx: &SyncSender<ShellMsg>) {
        let is_fg = self.fg.map_or(false, |x| pgid == x);
        let line = &self.jobs.get(&job_id).unwrap().1;

        if is_fg {
            if self.is_group_empty(pgid) {
                eprintln!("[{job_id}] 終了\t{line}");
                self.remove_job(job_id);
                self.set_shell_fg(shell_tx);
            } else if self.is_group_stop(pgid).unwrap() {
                eprintln!("\n[{job_id}] 停止\t{line}");
                self.set_shell_fg(shell_tx);
            }
        } else if self.is_group_empty(pgid) {
            eprintln!("[{job_id}] 終了\t{line}");
            self.remove_job(job_id);
        }
    }

    fn insert_job(&mut self, job_id: usize, pgid: Pid, pids: HashMap<Pid, ProcInfo>, line: &str) {
        assert!(!self.jobs.contains_key(&job_id));
        self.jobs.insert(job_id, (pgid, line.to_string()));

        let mut procs = HashSet::new();
        for (pid, info) in pids {
            procs.insert(pid);

            assert!(!self.pid_to_info.contains_key(&pid));
            self.pid_to_info.insert(pid, info);
        }

        assert!(!self.pid_to_info.contains_key(&pgid));
        self.pgid_to_pids.insert(pgid, (job_id, procs));
    }

    fn set_pid_state(&mut self, pid: Pid, state: ProcState) -> Option<ProcState> {
        let info: &mut ProcInfo = self.pid_to_info.get_mut(&pid)?;
        Some(replace(&mut info.state, state))
    }

    fn remove_pid(&mut self, pid: Pid) -> Option<(usize, Pid)> {
        let pgid = self.pid_to_info.get(&pid)?.pgid;
        let it = self.pgid_to_pids.get_mut(&pgid)?;
        it.1.remove(&pid);
        let job_id = it.0;
        Some((job_id, pgid))
    }

    fn remove_job(&mut self, job_id: usize) {
        if let Some((pgid, _)) = self.jobs.remove(&job_id) {
            if let Some((_, pids)) = self.pgid_to_pids.remove(&pgid) {
                assert!(pids.is_empty());
            }
        }
    }

    fn is_group_empty(&self, pgid: Pid) -> bool {
        self.pgid_to_pids.get(&pgid).unwrap().1.is_empty()
    }

    fn is_group_stop(&self, pgid: Pid) -> Option<bool> {
        for pid in self.pgid_to_pids.get(&pgid)?.1.iter() {
            if self.pid_to_info.get(pid).unwrap().state == ProcState::Run {
                return Some(false);
            }
        }
        Some(true)
    }

    fn set_shell_fg(&mut self, shell_tx: &SyncSender<ShellMsg>) {
        self.fg = None;
        tcsetpgrp(libc::STDIN_FILENO, self.shell_pgid).unwrap();
        shell_tx.send(ShellMsg::Continue(self.exit_val)).unwrap();
    }

    fn get_new_job_id(&self) -> Option<usize> {
        for i in 0..=usize::MAX {
            if !self.jobs.contains_key(&i) {
                return Some(i);
            }
        }
        None
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

fn fork_exec(
    pgid: Pid,
    filename: &str,
    args: &[&str],
    input: Option<i32>,
    output: Option<i32>,
) -> Result<Pid, DynError> {
    let filename = CString::new(filename).unwrap();
    let args: Vec<CString> = args.iter().map(|s| CString::new(*s).unwrap()).collect();

    match syscall(|| unsafe { fork() })? {
        ForkResult::Parent { child, .. } => {
            setpgid(child, pgid).unwrap();
            Ok(child)
        }
        ForkResult::Child => {
            setpgid(Pid::from_raw(0), pgid).unwrap();
            if let Some(fd) = input {
                syscall(|| dup2(fd, libc::STDIN_FILENO)).unwrap();
            };
            if let Some(fd) = output {
                syscall(|| dup2(fd, libc::STDOUT_FILENO)).unwrap();
            };

            for i in 3..=6 {
                let _ = syscall(|| unistd::close(i));
            }

            match execvp(&filename, &args) {
                Ok(_) => unreachable!(),
                Err(_) => {
                    unistd::write(libc::STDERR_FILENO, "不明なコマンドを実行\n".as_bytes()).ok();
                    exit(1);
                }
            }
        }
    }
}
