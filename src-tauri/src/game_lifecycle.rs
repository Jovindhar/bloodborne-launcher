// SPDX-License-Identifier: GPL-2.0-or-later
use std::{
    io,
    process::{Child, ExitStatus},
};

#[derive(Debug, PartialEq)]
pub enum Poll {
    Running,
    Finished(Option<i32>),
    Failed(String),
}
fn classify(result: io::Result<Option<ExitStatus>>) -> Poll {
    match result {
        Ok(None) => Poll::Running,
        Ok(Some(status)) => Poll::Finished(status.code()),
        Err(error) => Poll::Failed(error.to_string()),
    }
}
pub fn poll(child: Option<&mut Child>) -> Poll {
    child.map_or(Poll::Finished(None), |child| classify(child.try_wait()))
}
// The tests start sh and sleep.
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::process::Command;
    #[test]
    fn normal_and_failed_processes_finish_with_their_exit_codes() {
        for code in [0, 7] {
            let mut child = Command::new("sh")
                .args(["-c", &format!("exit {code}")])
                .spawn()
                .unwrap();
            child.wait().unwrap();
            assert_eq!(poll(Some(&mut child)), Poll::Finished(Some(code)));
        }
    }
    #[test]
    fn live_and_killed_processes_are_reported_without_hanging() {
        let mut child = Command::new("sleep").arg("30").spawn().unwrap();
        assert_eq!(poll(Some(&mut child)), Poll::Running);
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(matches!(poll(Some(&mut child)), Poll::Finished(_)));
    }
    #[test]
    fn missing_handle_still_finishes() {
        assert_eq!(poll(None), Poll::Finished(None));
    }
    #[test]
    fn wait_error_is_not_mistaken_for_process_exit() {
        assert!(matches!(
            classify(Err(io::ErrorKind::Interrupted.into())),
            Poll::Failed(_)
        ));
    }
}
