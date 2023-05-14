use std::sync::mpsc::SyncSender;

use nix::{
    libc,
    sys::signal::{killpg, Signal},
    unistd::tcsetpgrp,
};

use super::{shell_main::ShellMsg, worker::Worker};

pub fn built_in_cmd(
    worker: &mut Worker,
    cmd: &[(&str, Vec<&str>)],
    shell_tx: &SyncSender<ShellMsg>,
) -> bool {
    if cmd.len() > 1 {
        return false;
    }

    match cmd[0].0 {
        "exit" => run_exit(worker, &cmd[0].1, shell_tx),
        "jobs" => todo!(),
        "fg" => run_fg(worker, &cmd[0].1, shell_tx),
        "cd" => todo!(),
        _ => false,
    }
}

fn run_exit(worker: &mut Worker, args: &[&str], shell_tx: &SyncSender<ShellMsg>) -> bool {
    if !worker.jobs.is_empty() {
        eprintln!("ジョブが実行中なので終了できません");
        worker.exit_val = 1;
        shell_tx.send(ShellMsg::Continue(worker.exit_val)).unwrap();
        return true;
    }

    let exit_val = match args.get(1) {
        Some(s) => match (*s).parse::<i32>() {
            Ok(n) => n,
            _ => {
                eprintln!("{s}は不正な引数です");
                worker.exit_val = 1;
                shell_tx.send(ShellMsg::Continue(worker.exit_val)).unwrap();
                return true;
            }
        },
        None => worker.exit_val,
    };

    shell_tx.send(ShellMsg::Quit(exit_val)).unwrap();
    true
}

fn run_fg(worker: &mut Worker, args: &[&str], shell_tx: &SyncSender<ShellMsg>) -> bool {
    worker.exit_val = 1;

    if args.len() < 2 {
        eprintln!("usage: fg 数字");
        shell_tx.send(ShellMsg::Continue(worker.exit_val)).unwrap();
        return true;
    }

    if let Ok(n) = args[1].parse::<usize>() {
        if let Some((pgid, cmd)) = worker.jobs.get(&n) {
            eprintln!("[{n}] 再開\t{cmd}");

            worker.fg = Some(*pgid);
            tcsetpgrp(libc::STDIN_FILENO, *pgid).unwrap();

            killpg(*pgid, Signal::SIGCONT).unwrap();
            return true;
        }
    }

    eprintln!("{}というジョブは見つかりませんでした", args[1]);
    shell_tx.send(ShellMsg::Continue(worker.exit_val)).unwrap();
    true
}
