#![cfg(feature = "cyw43")]
use super::super::wlan::*;
use crate::apps::shell::ShellApp;
use crate::sys::task::Priority;

const PING_PRIO: u8 = 100;
const PING_STACK_SIZE: usize = 256;

extern "C" fn ping_task(_arg: *mut ()) {
    WLAN_CMD_QUEUE.send(WlanCmd::Ping);
}

pub(super) static PING_APP: ShellApp = ShellApp::new(
    "ping",
    "Ping gateway",
    ping_task,
    PING_STACK_SIZE,
    Priority(PING_PRIO),
);
