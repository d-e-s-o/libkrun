use macros::{guest, host};

pub struct TestSigintBrokenPipe;

#[host]
mod host {
    use super::*;

    use crate::{krun_call, krun_call_u32};
    use crate::{Test, TestSetup};
    use krun_sys::*;
    use nix::sys::signal;
    use nix::unistd::Pid;
    use std::process::Child;
    use std::ptr::null;
    use std::thread;
    use std::time::{Duration, Instant};

    impl Test for TestSigintBrokenPipe {
        fn start_vm(self: Box<Self>, _test_setup: TestSetup) -> anyhow::Result<()> {
            unsafe {
                krun_call!(krun_set_log_level(KRUN_LOG_LEVEL_TRACE))?;
                let ctx = krun_call_u32!(krun_create_ctx())?;
                krun_call!(krun_set_vm_config(ctx, 1, 256))?;
                krun_call!(krun_set_root(ctx, c"/".as_ptr()))?;
                krun_call!(krun_set_workdir(ctx, c"/".as_ptr()))?;
                // Run a command that writes to stdout continuously. This
                // ensures the TX thread is actively writing when the pipe
                // breaks, which triggers the spin loop that the fix
                // addresses.
                let argv = [
                    c"-c".as_ptr(),
                    c"while true; do echo x; done".as_ptr(),
                    null(),
                ];
                let envp = [null()];
                krun_call!(krun_set_exec(
                    ctx,
                    c"/bin/sh".as_ptr(),
                    argv.as_ptr(),
                    envp.as_ptr(),
                ))?;
                krun_call!(krun_start_enter(ctx))?;
            }
            Ok(())
        }

        fn check(self: Box<Self>, mut child: Child) {
            // Wait for the VM to boot and the SIGINT handler to be registered.
            thread::sleep(Duration::from_secs(3));

            // Break the pipes so host-side I/O threads encounter broken-pipe errors.
            drop(child.stdin.take());
            drop(child.stdout.take());

            // Send SIGINT to the process (simulates Ctrl+C / parent death).
            let pid = Pid::from_raw(child.id() as i32);
            signal::kill(pid, signal::Signal::SIGINT).expect("kill(SIGINT) failed");

            // Poll for exit with a 10 s deadline.
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                match child.try_wait().expect("try_wait failed") {
                    Some(_status) => return,
                    None if Instant::now() >= deadline => {
                        let _ignore = child.kill();
                        let _ignore = child.wait();
                        panic!(
                            "process did not exit within 10 s after SIGINT + broken pipes \
                             — likely hanging"
                        );
                    }
                    None => thread::sleep(Duration::from_millis(100)),
                }
            }
        }
    }
}

#[guest]
mod guest {
    use super::*;
    use crate::Test;

    impl Test for TestSigintBrokenPipe {}
}
