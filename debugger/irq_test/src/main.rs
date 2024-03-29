use std::arch::asm;

use nix::{
    sys::signal::{kill, Signal},
    unistd::getpid,
};

fn main() {
    println!("int 3");
    unsafe { asm!("int 3") }; //デバッグ実行するとここの実行後に止まる

    println!("kill -SIGTRAP");
    let pid = getpid();
    kill(pid, Signal::SIGTRAP).unwrap();

    for i in 0..3 {
        unsafe { asm!("nop") };
        println!("i = {i}");
    }
}
