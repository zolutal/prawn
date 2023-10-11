pub mod context;
pub mod process;
pub mod logging;
pub mod timer;
pub mod util;

#[cfg(test)]
mod tests {
    use process::ProcessConfig;
    use process::tubes::Tube;
    use logging as log;
    use context;

    use std::time::Duration;

    use super::*;

    #[test]
    fn echo_recv() -> anyhow::Result<()> {
        let cfg = ProcessConfig::default();
        let mut p = process::Process::new(["/bin/echo", "testing"], &cfg)?;

        let output = p.recv(4)?;
        let output_str: String = String::from_utf8(output)?;
        log::info(format!("echo_recv: {}", output_str));
        assert!(output_str == "test");

        let output = p.recv(4)?;
        let output_str: String = String::from_utf8(output)?;
        log::info(format!("echo_recv: {}", output_str));
        assert!(output_str == "ing\n");

        // should timeout
        let output = p.recv_timeout(1, timer::TimeoutVal::Duration(
                                       Duration::from_millis(500)));
        log::info(format!("echo_recv: {:?}", output));
        assert!(output.is_err());

        Ok(())
    }

    #[test]
    fn cat_send_recv() -> anyhow::Result<()> {
        let cfg = process::ProcessConfig::default();
        let mut p = process::Process::new(["/bin/cat"], &cfg)?;
        let _ = p.sendline("testing!\n".into())?;
        let data = p.recv(9)?;
        let output: String = String::from_utf8(data)?;
        log::info(format!("cat_send_recv: {}", output));
        assert!(output == "testing!\n");
        Ok(())
    }

    #[test]
    fn cat_recvuntil() -> anyhow::Result<()> {
        let cfg = process::ProcessConfig::default();
        let mut p = process::Process::new(["/bin/cat"], &cfg)?;

        p.sendline("testing!".into())?;
        p.sendline("testing2!".into())?;

        let data = p.recvuntil("!".into())?;
        let output: String = String::from_utf8(data)?;
        log::info(format!("cat_recvuntil: {}", output));
        assert!(output == "testing!");

        // skip pending newline
        let _ = p.recv(1)?;

        let data = p.recvline()?;
        let output: String = String::from_utf8(data)?;
        log::info(format!("cat_recvuntil: {}", output));
        assert!(output == "testing2!\n");

        p.sendline("testing3!\n".into())?;

        let data = p.recvline()?;
        let output: String = String::from_utf8(data)?;
        log::info(format!("cat_recvuntil: {}", output));
        assert!(output == "testing3!\n");

        Ok(())
    }

    #[test]
    fn pack_unpack() -> anyhow::Result<()> {
        let packed = util::p64(0xdeadbeefcafebeef)?;
        let unpacked = util::u64(&packed)?;
        log::info(format!("{:?}",  &packed));
        log::info(format!("{:#x}", &unpacked));
        assert!(unpacked == 0xdeadbeefcafebeef);
        Ok(())
    }

    #[test]
    fn test_context() -> anyhow::Result<()> {
        context::access(|ctx| {
            ctx.aslr = false;
        });
        let aslr = context::access(|ctx| {
            ctx.aslr
        });
        assert!(!aslr);
        Ok(())
    }

    #[test]
    fn test_logging() -> anyhow::Result<()> {
        log::debug("should not print");
        context::access(|ctx| {
            ctx.log_level = log::LogLevel::Debug;
        });
        log::debug("should print");
        log::info("info");
        log::warn("warn");
        log::error("error");
        log::critical("crit");
        Ok(())
    }
}
