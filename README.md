# prawn

A library for pwning written in Rust based on pwntools

# Currently Implemented:

* processes spawning and interaction
* global context configuration
* basic logging
* basic packing (only p64/u64)

# TODO

missing pwntools stuff:

* support remote targets
* gdb debug/attach
* goblin wrapper for ELF and PE parsers
* ROP stuff on top of ropr?
* assemble/disassemble
* shellcode generation
* cyclics
* core files
* command line sutff?
* hexdump
* checksec
* fmtstr
* FSOP, SROP, ret2dlresolve

other:

* kernel pwn primitives
* support vm targets
* uarch primitives

# Why??

I've been enjoying Rust lately and this seemed like a fun waste of time

Prior attempts at doing this have not implemented all the features I care about and I'd rather write my own then fork/try to revive those

# Examples

An example solve for 'super\_secure\_heap' from CSAW quals 2023 can be found in the demos directory
