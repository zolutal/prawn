# prawn

A library for binary exploitation written in Rust inspired by pwntools

# Currently Implemented:

* processes spawning and interaction
* global context configuration
* basic logging
* basic byte (un)packing (p64/u64 etc...)
* support remote targets

# TODO

missing pwntools stuff:

* goblin wrapper for ELF and PE parsers (WIP)
* FSOP (WIP), SROP, ret2dlresolve
* fmtstr (WIP)
* gdb debug/attach
* ROP stuff on top of ropr?
* assemble/disassemble
* shellcode generation
* cyclics
* core files
* command line stuff?
* hexdump
* checksec

# Why??

I was having a good time with Rust and this seemed like a fun waste of time

Prior attempts at doing this had not implemented all the features I care about and I'd rather write my own then fork/try to revive those

# Examples

An example solve for 'super\_secure\_heap' from CSAW quals 2023 can be found in the demos directory
