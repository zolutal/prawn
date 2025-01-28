pub mod context;
pub mod tubes;
pub mod logging;
pub mod timer;
pub mod heap;
pub mod util;
pub mod elf;

#[cfg(test)]
mod tests {
    use std::{io::Write, path::PathBuf, process::Command, fs::File};
    use tubes::process::*;
    use logging as log;
    use elf::Elf;
    use memmap2::MmapOptions;
    use tempfile::TempDir;
    use tubes::Tube;

    use std::time::Duration;

    use super::*;

    #[tokio::test]
    async fn echo_recv() -> anyhow::Result<()> {
        let cfg = ProcessConfig::default();
        let mut p = Process::new(["/bin/echo", "testing"], &cfg).await?;

        let output = p.recv(4).await?;
        let output_str: String = String::from_utf8(output)?;
        log::info(format!("echo_recv: {}", output_str));
        assert!(output_str == "test");

        let output = p.recv(4).await?;
        let output_str: String = String::from_utf8(output)?;
        log::info(format!("echo_recv: {}", output_str));
        assert!(output_str == "ing\n");

        // should timeout
        let output = p.recv_timeout(
            1,
            timer::TimeoutVal::Duration(Duration::from_millis(500))
        ).await;
        log::info(format!("echo_recv: {:?}", output));
        assert!(output.is_ok());

        Ok(())
    }

    #[tokio::test]
    async fn cat_send_recv() -> anyhow::Result<()> {
        let cfg = ProcessConfig::default();
        let mut p = Process::new(["/bin/cat"], &cfg).await?;
        p.sendline(b"testing!\n").await?;
        let data = p.recv(9).await?;
        let output: String = String::from_utf8(data)?;
        log::info(format!("cat_send_recv: {}", output));
        assert!(output == "testing!\n");
        Ok(())
    }

    #[tokio::test]
    async fn cat_recvuntil() -> anyhow::Result<()> {
        let cfg = ProcessConfig::default();
        let mut p = Process::new(["/bin/cat"], &cfg).await?;

        p.sendline(b"testing!").await?;
        p.sendline(b"testing2!").await?;

        let data = p.recvuntil(b"!").await?;
        let output: String = String::from_utf8(data)?;
        log::info(format!("cat_recvuntil: {}", output));
        assert!(output == "testing!");

        // skip pending newline
        let _ = p.recv(1).await?;

        let data = p.recvline().await?;
        let output: String = String::from_utf8(data)?;
        log::info(format!("cat_recvuntil: {}", output));
        assert!(output == "testing2!\n");

        p.sendline(b"testing3!\n").await?;

        let data = p.recvline().await?;
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

    fn compile(source: &str) -> anyhow::Result<(TempDir, PathBuf)> {
        let tmp_dir = TempDir::new()?;
        let src_path = tmp_dir.path().join("src.c");

        {
            let mut tmp_file = File::create(&src_path)?;
            tmp_file.write_all(source.as_bytes())?;
        }

        let out_path = tmp_dir.path().join("bin");
        let output = Command::new("gcc")
            .arg(&src_path)
            .arg("-o")
            .arg(&out_path)
            .output()?;

        if !output.status.success() {
            panic!("gcc failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok((tmp_dir, out_path))
    }

    const SIMPLE: &str = "
    struct simple {
        unsigned long long s;
    };
    int main() {
        struct simple s;
    }";


    #[test]
    fn test_elf() -> anyhow::Result<()> {
        let (_tmpdir, path) = compile(SIMPLE)?;
        let file = File::open(path)?;
        let mmap = unsafe { MmapOptions::new().map(&file) }?;
        let elf = Elf::new(&mmap)?;

        dbg!(&elf.bits);
        dbg!(&elf.arch);

        let addr = elf.symbols.get("main").unwrap();
        println!("main: {addr:#x}");

        Ok(())
    }
}
