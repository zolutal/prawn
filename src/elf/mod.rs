use std::collections::HashMap;
use goblin::elf::Elf as GElf;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Goblin Error")]
    GoblinError(#[from] goblin::error::Error),

    #[error("Elf Parsing Error: {0}")]
    ElfParsingError(String)
}

fn collect_symbols(elf: &GElf) -> HashMap<String, u64> {
    let mut symbols_map: HashMap<String, u64> = HashMap::new();
    for sym in elf.syms.iter() {
        if sym.st_value != 0 {
            let name = &elf.strtab[sym.st_name];
            symbols_map.insert(name.to_string(), sym.st_value);
        }
    }
    symbols_map
}

#[derive(Clone, Debug)]
pub enum Arch {
    AMD64,
    I386,
    AARCH64,
    ARM,
    RISCV,
}

#[derive(Clone)]
pub struct Elf {
    pub symbols: HashMap<String, u64>,
    pub arch: Arch,
    pub bits: u64,
    pub bytes: u64,
}

impl Elf {
    pub fn new(data: &[u8]) -> Result<Elf, Error> {
        let elf = GElf::parse(data)?;

        let symbols = collect_symbols(&elf);

        let e_machine = elf.header.e_machine;
        let arch = match e_machine {
            3   => Arch::I386,
            40  => Arch::ARM,
            62  => Arch::AMD64,
            183 => Arch::AARCH64,
            243 => Arch::RISCV,
            _ => {
                return Err(Error::ElfParsingError(
                    format!("Unsupported Architecture {}", e_machine)
                ))
            }
        };

        let (bits, bytes) = match elf.header.e_ident[goblin::elf::header::EI_CLASS] {
            1 => (32, 4),
            2 => (64, 8),
            _ => {
                return Err(Error::ElfParsingError(
                    "Invalid bitness in Elf Header".to_string()
                ))
            }
        };

        // Return the Elf struct
        Ok(Self { symbols, arch, bits, bytes })
    }
}
