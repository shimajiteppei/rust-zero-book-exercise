use nix::{
    libc,
    sys::wait::{waitpid, WaitPidFlag, WaitStatus},
    unistd::{self, dup2, execvp, fork, pipe, setpgid, tcsetpgrp, ForkResult, Pid},
};
use std::{
    collections::{HashMap, HashSet},
    ffi::CString,
    mem::replace,
    process::exit,
    sync::mpsc::SyncSender,
};

use super::{
    shell_main::{DynError, ShellMsg},
    worker::{ProcInfo, ProcState, Worker},
};

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

pub fn spawn_child(worker: &mut Worker, line: &str, cmd: &[(&str, Vec<&str>)]) -> bool {
    assert_ne!(cmd.len(), 0);

    let job_id = match get_new_job_id(worker) {
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

    worker.fg = Some(pgid);
    insert_job(worker, job_id, pgid, pids, line);
    tcsetpgrp(libc::STDIN_FILENO, pgid).unwrap();

    true
}

pub fn wait_child(worker: &mut Worker, shell_tx: &SyncSender<ShellMsg>) {
    let flag = Some(WaitPidFlag::WUNTRACED | WaitPidFlag::WNOHANG | WaitPidFlag::WCONTINUED);

    loop {
        match syscall(|| waitpid(Pid::from_raw(-1), flag)) {
            Ok(WaitStatus::Exited(pid, status)) => {
                worker.exit_val = status;
                process_term(worker, pid, shell_tx);
            }
            Ok(WaitStatus::Signaled(pid, sig, core)) => {
                eprintln!(
                    "\nZeroSh: 子プロセスがシグナルにより終了{}: pid = {pid}, signal = {sig}",
                    if core { " (コアダンプ) " } else { "" }
                );
                worker.exit_val = sig as i32 + 128;
                process_term(worker, pid, shell_tx);
            }
            Ok(WaitStatus::Stopped(pid, _sig)) => {
                process_stop(worker, pid, shell_tx);
            }
            Ok(WaitStatus::Continued(pid)) => {
                process_continue(worker, pid);
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
                process_stop(worker, pid, shell_tx)
            }
        }
    }
}

fn process_term(worker: &mut Worker, pid: Pid, shell_tx: &SyncSender<ShellMsg>) {
    if let Some((job_id, pgid)) = remove_pid(worker, pid) {
        manage_job(worker, job_id, pgid, shell_tx);
    }
}

fn process_stop(worker: &mut Worker, pid: Pid, shell_tx: &SyncSender<ShellMsg>) {
    set_pid_state(worker, pid, ProcState::Stop);
    let pgid = worker.pid_to_info.get(&pid).unwrap().pgid;
    let job_id = worker.pgid_to_pids.get(&pgid).unwrap().0;
    manage_job(worker, job_id, pgid, shell_tx);
}

fn process_continue(worker: &mut Worker, pid: Pid) {
    set_pid_state(worker, pid, ProcState::Run);
}

fn manage_job(worker: &mut Worker, job_id: usize, pgid: Pid, shell_tx: &SyncSender<ShellMsg>) {
    let is_fg = worker.fg.map_or(false, |x| pgid == x);
    let line = &worker.jobs.get(&job_id).unwrap().1;

    if is_fg {
        if is_group_empty(worker, pgid) {
            eprintln!("[{job_id}] 終了\t{line}");
            remove_job(worker, job_id);
            set_shell_fg(worker, shell_tx);
        } else if is_group_stop(worker, pgid).unwrap() {
            eprintln!("\n[{job_id}] 停止\t{line}");
            set_shell_fg(worker, shell_tx);
        }
    } else if is_group_empty(worker, pgid) {
        eprintln!("[{job_id}] 終了\t{line}");
        remove_job(worker, job_id);
    }
}

fn insert_job(
    worker: &mut Worker,
    job_id: usize,
    pgid: Pid,
    pids: HashMap<Pid, ProcInfo>,
    line: &str,
) {
    assert!(!worker.jobs.contains_key(&job_id));
    worker.jobs.insert(job_id, (pgid, line.to_string()));

    let mut procs = HashSet::new();
    for (pid, info) in pids {
        procs.insert(pid);

        assert!(!worker.pid_to_info.contains_key(&pid));
        worker.pid_to_info.insert(pid, info);
    }

    assert!(!worker.pid_to_info.contains_key(&pgid));
    worker.pgid_to_pids.insert(pgid, (job_id, procs));
}

fn set_pid_state(worker: &mut Worker, pid: Pid, state: ProcState) -> Option<ProcState> {
    let info: &mut ProcInfo = worker.pid_to_info.get_mut(&pid)?;
    Some(replace(&mut info.state, state))
}

fn remove_pid(worker: &mut Worker, pid: Pid) -> Option<(usize, Pid)> {
    let pgid = worker.pid_to_info.get(&pid)?.pgid;
    let it = worker.pgid_to_pids.get_mut(&pgid)?;
    it.1.remove(&pid);
    let job_id = it.0;
    Some((job_id, pgid))
}

fn remove_job(worker: &mut Worker, job_id: usize) {
    if let Some((pgid, _)) = worker.jobs.remove(&job_id) {
        if let Some((_, pids)) = worker.pgid_to_pids.remove(&pgid) {
            assert!(pids.is_empty());
        }
    }
}

fn is_group_empty(worker: &Worker, pgid: Pid) -> bool {
    worker.pgid_to_pids.get(&pgid).unwrap().1.is_empty()
}

fn is_group_stop(worker: &Worker, pgid: Pid) -> Option<bool> {
    for pid in worker.pgid_to_pids.get(&pgid)?.1.iter() {
        if worker.pid_to_info.get(pid).unwrap().state == ProcState::Run {
            return Some(false);
        }
    }
    Some(true)
}

fn set_shell_fg(worker: &mut Worker, shell_tx: &SyncSender<ShellMsg>) {
    worker.fg = None;
    tcsetpgrp(libc::STDIN_FILENO, worker.shell_pgid).unwrap();
    shell_tx.send(ShellMsg::Continue(worker.exit_val)).unwrap();
}

fn get_new_job_id(worker: &Worker) -> Option<usize> {
    for i in 0..=usize::MAX {
        if !worker.jobs.contains_key(&i) {
            return Some(i);
        }
    }
    None
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
