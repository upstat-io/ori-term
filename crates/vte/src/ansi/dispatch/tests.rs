//! OSC 1337 sub-dispatcher parse pins.
//!
//! Verifies that the refactored `b"1337"` arm routes each sub-op to the
//! correct `Handler` method. The existing `iterm2_file` path stays wired;
//! the seven non-image sub-ops (`SetMark`, `RemoteHost=`, `CurrentDir=`,
//! `Copy=`, `ReportCellSize`, `SetUserVar=`, `ShellIntegrationVersion=`)
//! reach their own hooks.

extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use core::cell::RefCell;

use crate::ansi::Processor;
use crate::ansi::handler::Handler;

/// Records every iTerm2 sub-op the dispatcher routed to us.
#[derive(Default)]
struct RecordingHandler {
    set_mark_calls: u32,
    report_cell_size_calls: u32,
    remote_host: Vec<Vec<u8>>,
    current_dir: Vec<Vec<u8>>,
    copy: Vec<Vec<u8>>,
    set_user_var: Vec<(Vec<u8>, Vec<u8>)>,
    shell_integration_version: Vec<Vec<u8>>,
    iterm2_file_calls: u32,
}

impl Handler for RecordingHandler {
    fn iterm2_file(&mut self, _params: &[&[u8]]) {
        self.iterm2_file_calls += 1;
    }
    fn iterm2_set_mark(&mut self) {
        self.set_mark_calls += 1;
    }
    fn iterm2_remote_host(&mut self, host: &[u8]) {
        self.remote_host.push(host.to_vec());
    }
    fn iterm2_current_dir(&mut self, path: &[u8]) {
        self.current_dir.push(path.to_vec());
    }
    fn iterm2_copy(&mut self, data: &[u8]) {
        self.copy.push(data.to_vec());
    }
    fn iterm2_report_cell_size(&mut self) {
        self.report_cell_size_calls += 1;
    }
    fn iterm2_set_user_var(&mut self, name: &[u8], value: &[u8]) {
        self.set_user_var.push((name.to_vec(), value.to_vec()));
    }
    fn iterm2_shell_integration_version(&mut self, version: &[u8]) {
        self.shell_integration_version.push(version.to_vec());
    }
}

fn feed(bytes: &[u8]) -> RecordingHandler {
    let cell = RefCell::new(RecordingHandler::default());
    {
        let mut processor: Processor = Processor::new();
        let mut handler = cell.borrow_mut();
        processor.advance(&mut *handler, bytes);
    }
    cell.into_inner()
}

/// OSC 1337 ; SetMark routes to `Handler::iterm2_set_mark`.
#[test]
fn osc1337_set_mark_routes_to_iterm2_set_mark() {
    let h = feed(b"\x1b]1337;SetMark\x1b\\");
    assert_eq!(h.set_mark_calls, 1);
    assert_eq!(h.iterm2_file_calls, 0);
}

/// OSC 1337 ; ReportCellSize routes to `Handler::iterm2_report_cell_size`.
#[test]
fn osc1337_report_cell_size_routes_to_iterm2_report_cell_size() {
    let h = feed(b"\x1b]1337;ReportCellSize\x1b\\");
    assert_eq!(h.report_cell_size_calls, 1);
}

/// OSC 1337 ; RemoteHost=user@host routes to `Handler::iterm2_remote_host`
/// with the host payload.
#[test]
fn osc1337_remote_host_routes_with_payload() {
    let h = feed(b"\x1b]1337;RemoteHost=user@host.example.com\x1b\\");
    assert_eq!(h.remote_host.len(), 1);
    assert_eq!(
        String::from_utf8_lossy(&h.remote_host[0]),
        "user@host.example.com"
    );
}

/// OSC 1337 ; CurrentDir=/path routes to `Handler::iterm2_current_dir`.
#[test]
fn osc1337_current_dir_routes_with_payload() {
    let h = feed(b"\x1b]1337;CurrentDir=/home/user/project\x1b\\");
    assert_eq!(h.current_dir.len(), 1);
    assert_eq!(
        String::from_utf8_lossy(&h.current_dir[0]),
        "/home/user/project"
    );
}

/// OSC 1337 ; Copy=:<base64> routes to `Handler::iterm2_copy`.
#[test]
fn osc1337_copy_routes_with_payload() {
    let h = feed(b"\x1b]1337;Copy=:SGVsbG8=\x1b\\");
    assert_eq!(h.copy.len(), 1);
    assert_eq!(String::from_utf8_lossy(&h.copy[0]), ":SGVsbG8=");
}

/// OSC 1337 ; SetUserVar=NAME=<base64> splits payload into name + value.
#[test]
fn osc1337_set_user_var_splits_name_and_value() {
    let h = feed(b"\x1b]1337;SetUserVar=MY_VAR=SGVsbG8=\x1b\\");
    assert_eq!(h.set_user_var.len(), 1);
    let (name, value) = &h.set_user_var[0];
    assert_eq!(String::from_utf8_lossy(name), "MY_VAR");
    assert_eq!(String::from_utf8_lossy(value), "SGVsbG8=");
}

/// OSC 1337 ; ShellIntegrationVersion=N routes with the version payload.
#[test]
fn osc1337_shell_integration_version_routes_with_payload() {
    let h = feed(b"\x1b]1337;ShellIntegrationVersion=5\x1b\\");
    assert_eq!(h.shell_integration_version.len(), 1);
    assert_eq!(String::from_utf8_lossy(&h.shell_integration_version[0]), "5");
}

/// OSC 1337 ; File=... still routes to `Handler::iterm2_file` so Section 14's
/// image path is unaffected by the sub-dispatcher refactor.
#[test]
fn osc1337_file_still_routes_to_iterm2_file() {
    let h = feed(b"\x1b]1337;File=name=test.png;size=42:QUJD\x1b\\");
    assert_eq!(h.iterm2_file_calls, 1);
    assert_eq!(h.set_mark_calls, 0);
}

/// OSC 1337 with an unknown sub-op is silently dropped — no panic, no
/// routing.
#[test]
fn osc1337_unknown_sub_op_is_dropped() {
    let h = feed(b"\x1b]1337;NotARealKey=value\x1b\\");
    assert_eq!(h.set_mark_calls, 0);
    assert_eq!(h.iterm2_file_calls, 0);
    assert_eq!(h.remote_host.len(), 0);
}
