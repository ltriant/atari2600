use std::env;
use std::process;

use crate::bus::Bus;

const STACK_INIT: u8 = 0xff;

lazy_static!{
    static ref CPU6507_DEBUG: bool = match env::var("CPU6507_DEBUG") {
        Ok(val) => val != "" && val != "0",
        Err(_)  => false,
    };
}

#[derive(Copy, Clone, Debug)]
pub enum Instruction {
    None,
    ADC, ANC, AND, ASL, BCC, BCS, BEQ, BIT,
    BMI, BNE, BPL, BRK, BVC, BVS, CLC, CLD,
    CLI, CLV, CMP, CPX, CPY, DCP, DEC, DEX,
    DEY, EOR, INC, INX, INY, ISB, JAM, JMP,
    JSR, LAX, LDA, LDX, LDY, LSR, NOP, ORA,
    PHA, PHP, PLA, PLP, RLA, ROL, ROR, RRA,
    RTI, RTS, SAX, SBC, SEC, SED, SEI, SLO,
    SRE, STA, STX, STY, TAX, TAY, TSX, TXA,
    TXS, TYA,
}

#[derive(Copy, Clone, Debug)]
pub enum AddressingMode {
    None,
    Immediate,
    Absolute,
    Implied,
    Accumulator,
    AbsoluteX,
    AbsoluteY,
    ZeroPageIndexed,
    ZeroPageX,
    ZeroPageY,
    Indirect,
    IndexedIndirect,
    IndirectIndexed,
    Relative,
}

fn pages_differ(addr_a: u16, addr_b: u16) -> bool {
    (addr_a & 0xff00) != (addr_b & 0xff00)
}

impl AddressingMode {
    pub fn n_bytes(&self) -> usize {
        match *self {
              AddressingMode::Implied
            | AddressingMode::Accumulator => 1,

              AddressingMode::Immediate
            | AddressingMode::ZeroPageIndexed
            | AddressingMode::Relative
            | AddressingMode::ZeroPageX
            | AddressingMode::ZeroPageY
            | AddressingMode::IndexedIndirect
            | AddressingMode::IndirectIndexed => 2,

              AddressingMode::Absolute
            | AddressingMode::AbsoluteX
            | AddressingMode::AbsoluteY
            | AddressingMode::Indirect => 3,

            _ => panic!("Bad addressing mode {:?}", *self),
        }
    }

    pub fn get_bytes(&self, cpu: &mut CPU6507) -> Vec<u8> {
        let n_bytes = self.n_bytes() as u16;
        (0 .. n_bytes).map(|n| cpu.read(cpu.pc + n)).collect::<Vec<_>>()
    }

    pub fn get_data(&self, cpu: &mut CPU6507) -> (u16, bool) {
        let pc = cpu.pc;
        let next_pc = cpu.pc + self.n_bytes() as u16;

        match *self {
            AddressingMode::Immediate => {
                let addr = pc + 1;
                (addr, false)
            },
            AddressingMode::Absolute => {
                let lo = cpu.read(pc + 1) as u16;
                let hi = cpu.read(pc + 2) as u16;
                let addr = (hi << 8) | lo;
                (addr, false)
            },
            AddressingMode::Implied => (0, false),
            AddressingMode::Accumulator => (0, false),
            AddressingMode::ZeroPageIndexed => {
                let addr = cpu.read(pc + 1) as u16;
                (addr, false)
            },
            AddressingMode::Relative => {
                let offset = cpu.read(pc + 1) as u16;

                // NOTE This has to be based off the program counter, _after_
                // it has been advanced, but before the instruction is
                // being executed. I don't know why though?

                // All of this casting is to handle negative offsets
                (((next_pc as i16) + (offset as i8 as i16)) as u16, false)
            },
            AddressingMode::AbsoluteX => {
                let lo = cpu.read(pc + 1) as u16;
                let hi = cpu.read(pc + 2) as u16;
                let addr = (hi << 8) | lo;
                let n_addr = addr.wrapping_add(cpu.x as u16);
                (n_addr, pages_differ(addr, n_addr))
            },
            AddressingMode::AbsoluteY => {
                let lo = cpu.read(pc + 1) as u16;
                let hi = cpu.read(pc + 2) as u16;
                let addr = (hi << 8) | lo;
                let n_addr = addr.wrapping_add(cpu.y as u16);
                (n_addr, pages_differ(addr, n_addr))
            },
            AddressingMode::Indirect => {
                let lo = cpu.read(pc + 1) as u16;
                let hi = cpu.read(pc + 2) as u16;
                let addr = (hi << 8) | lo;

                let lo = cpu.read(addr) as u16;

                let hi =
                    if addr & 0xff == 0xff {
                        cpu.read(addr & 0xff00) as u16
                    } else {
                        cpu.read(addr + 1) as u16
                    };

                let addr = (hi << 8) | lo;

                (addr, false)
            }
            AddressingMode::ZeroPageX => {
                let addr = cpu.read(pc + 1)
                    .wrapping_add(cpu.x) as u16;
                (addr, false)
            },
            AddressingMode::ZeroPageY => {
                let addr = cpu.read(pc + 1)
                    .wrapping_add(cpu.y) as u16;
                (addr, false)
            },
            AddressingMode::IndexedIndirect => {
                let lo = cpu.read(pc + 1);
                let addr = lo.wrapping_add(cpu.x) as u16;

                let lo = cpu.read(addr) as u16;

                let hi =
                    if addr & 0xff == 0xff {
                        cpu.read(addr & 0xff00) as u16
                    } else {
                        cpu.read(addr + 1) as u16
                    };

                let addr = (hi << 8) | lo;
                (addr, false)
            },
            AddressingMode::IndirectIndexed => {
                let addr = cpu.read(pc + 1) as u16;

                let lo = cpu.read(addr) as u16;

                let hi =
                    if addr & 0xff == 0xff {
                        cpu.read(addr & 0xff00) as u16
                    } else {
                        cpu.read(addr + 1) as u16
                    };

                let addr = (hi << 8) | lo;
                let n_addr = addr.wrapping_add(cpu.y as u16);

                (n_addr, pages_differ(addr, n_addr))
            },

            _ => panic!("Bad addressing mode {:?}", *self)
        }
    }
}

#[derive(Copy, Clone, Debug)]
struct Opcode(Instruction,
              AddressingMode,
              u64,   // number of cycles
              u64);  // number of extra cycles, if a page boundary is crossed

const OPCODES: [Opcode; 256] = [
    // 0x00
    Opcode(Instruction::BRK, AddressingMode::Implied, 7, 0),
    Opcode(Instruction::ORA, AddressingMode::IndexedIndirect, 6, 0),
    Opcode(Instruction::JAM, AddressingMode::Implied, 0, 0),
    Opcode(Instruction::SLO, AddressingMode::IndexedIndirect, 8, 0),
    Opcode(Instruction::NOP, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::ORA, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::ASL, AddressingMode::ZeroPageIndexed, 5, 0),
    Opcode(Instruction::SLO, AddressingMode::ZeroPageIndexed, 5, 0),
    Opcode(Instruction::PHP, AddressingMode::Implied, 3, 0),
    Opcode(Instruction::ORA, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::ASL, AddressingMode::Accumulator, 2, 0),
    Opcode(Instruction::ANC, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::NOP, AddressingMode::Absolute, 4, 0),
    Opcode(Instruction::ORA, AddressingMode::Absolute, 4, 0),
    Opcode(Instruction::ASL, AddressingMode::Absolute, 6, 0),
    Opcode(Instruction::SLO, AddressingMode::Absolute, 6, 0),

    // 0x10
    Opcode(Instruction::BPL, AddressingMode::Relative, 2, 1),
    Opcode(Instruction::ORA, AddressingMode::IndirectIndexed, 5, 1),
    Opcode(Instruction::JAM, AddressingMode::Implied, 0, 0),
    Opcode(Instruction::SLO, AddressingMode::IndirectIndexed, 8, 0),
    Opcode(Instruction::NOP, AddressingMode::ZeroPageX, 4, 0),
    Opcode(Instruction::ORA, AddressingMode::ZeroPageX, 4, 0),
    Opcode(Instruction::ASL, AddressingMode::ZeroPageX, 6, 0),
    Opcode(Instruction::SLO, AddressingMode::ZeroPageX, 6, 0),
    Opcode(Instruction::CLC, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::ORA, AddressingMode::AbsoluteY, 4, 1),
    Opcode(Instruction::NOP, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::SLO, AddressingMode::AbsoluteY, 7, 0),
    Opcode(Instruction::NOP, AddressingMode::AbsoluteX, 4, 1),
    Opcode(Instruction::ORA, AddressingMode::AbsoluteX, 4, 1),
    Opcode(Instruction::ASL, AddressingMode::AbsoluteX, 7, 0),
    Opcode(Instruction::SLO, AddressingMode::AbsoluteX, 7, 0),

    // 0x20
    Opcode(Instruction::JSR, AddressingMode::Absolute, 6, 0),
    Opcode(Instruction::AND, AddressingMode::IndexedIndirect, 6, 0),
    Opcode(Instruction::JAM, AddressingMode::Implied, 0, 0),
    Opcode(Instruction::RLA, AddressingMode::IndexedIndirect, 8, 0),
    Opcode(Instruction::BIT, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::AND, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::ROL, AddressingMode::ZeroPageIndexed, 5, 0),
    Opcode(Instruction::RLA, AddressingMode::ZeroPageIndexed, 5, 0),
    Opcode(Instruction::PLP, AddressingMode::Implied, 4, 0),
    Opcode(Instruction::AND, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::ROL, AddressingMode::Accumulator, 2, 0),
    Opcode(Instruction::ANC, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::BIT, AddressingMode::Absolute, 4, 0),
    Opcode(Instruction::AND, AddressingMode::Absolute, 4, 0),
    Opcode(Instruction::ROL, AddressingMode::Absolute, 6, 0),
    Opcode(Instruction::RLA, AddressingMode::Absolute, 6, 0),

    // 0x30
    Opcode(Instruction::BMI, AddressingMode::Relative, 2, 1),
    Opcode(Instruction::AND, AddressingMode::IndirectIndexed, 5, 1),
    Opcode(Instruction::JAM, AddressingMode::Implied, 0, 0),
    Opcode(Instruction::RLA, AddressingMode::IndirectIndexed, 8, 0),
    Opcode(Instruction::NOP, AddressingMode::ZeroPageX, 4, 0),
    Opcode(Instruction::AND, AddressingMode::ZeroPageX, 4, 0),
    Opcode(Instruction::ROL, AddressingMode::ZeroPageX, 6, 0),
    Opcode(Instruction::RLA, AddressingMode::ZeroPageX, 6, 0),
    Opcode(Instruction::SEC, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::AND, AddressingMode::AbsoluteY, 4, 1),
    Opcode(Instruction::NOP, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::RLA, AddressingMode::AbsoluteY, 7, 0),
    Opcode(Instruction::NOP, AddressingMode::AbsoluteX, 4, 1),
    Opcode(Instruction::AND, AddressingMode::AbsoluteX, 4, 1),
    Opcode(Instruction::ROL, AddressingMode::AbsoluteX, 7, 0),
    Opcode(Instruction::RLA, AddressingMode::AbsoluteX, 7, 0),

    // 0x40
    Opcode(Instruction::RTI, AddressingMode::Implied, 6, 0),
    Opcode(Instruction::EOR, AddressingMode::IndexedIndirect, 6, 0),
    Opcode(Instruction::JAM, AddressingMode::Implied, 0, 0),
    Opcode(Instruction::SRE, AddressingMode::IndexedIndirect, 8, 0),
    Opcode(Instruction::NOP, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::EOR, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::LSR, AddressingMode::ZeroPageIndexed, 5, 0),
    Opcode(Instruction::SRE, AddressingMode::ZeroPageIndexed, 5, 0),
    Opcode(Instruction::PHA, AddressingMode::Implied, 3, 0),
    Opcode(Instruction::EOR, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::LSR, AddressingMode::Accumulator, 2, 0),
    Opcode(Instruction::None, AddressingMode::None, 0, 0),
    Opcode(Instruction::JMP, AddressingMode::Absolute, 3, 0),
    Opcode(Instruction::EOR, AddressingMode::Absolute, 4, 0),
    Opcode(Instruction::LSR, AddressingMode::Absolute, 6, 0),
    Opcode(Instruction::SRE, AddressingMode::Absolute, 6, 0),

    // 0x50
    Opcode(Instruction::BVC, AddressingMode::Relative, 2, 1),
    Opcode(Instruction::EOR, AddressingMode::IndirectIndexed, 5, 1),
    Opcode(Instruction::JAM, AddressingMode::Implied, 0, 0),
    Opcode(Instruction::SRE, AddressingMode::IndirectIndexed, 8, 0),
    Opcode(Instruction::NOP, AddressingMode::ZeroPageX, 4, 0),
    Opcode(Instruction::EOR, AddressingMode::ZeroPageX, 4, 0),
    Opcode(Instruction::LSR, AddressingMode::ZeroPageX, 6, 0),
    Opcode(Instruction::SRE, AddressingMode::ZeroPageX, 6, 0),
    Opcode(Instruction::CLI, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::EOR, AddressingMode::AbsoluteY, 4, 1),
    Opcode(Instruction::NOP, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::SRE, AddressingMode::AbsoluteY, 7, 0),
    Opcode(Instruction::NOP, AddressingMode::AbsoluteX, 4, 1),
    Opcode(Instruction::EOR, AddressingMode::AbsoluteX, 4, 1),
    Opcode(Instruction::LSR, AddressingMode::AbsoluteX, 7, 0),
    Opcode(Instruction::SRE, AddressingMode::AbsoluteX, 7, 0),

    // 0x60
    Opcode(Instruction::RTS, AddressingMode::Implied, 6, 0),
    Opcode(Instruction::ADC, AddressingMode::IndexedIndirect, 6, 0),
    Opcode(Instruction::JAM, AddressingMode::Implied, 0, 0),
    Opcode(Instruction::RRA, AddressingMode::IndexedIndirect, 8, 0),
    Opcode(Instruction::NOP, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::ADC, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::ROR, AddressingMode::ZeroPageIndexed, 5, 0),
    Opcode(Instruction::RRA, AddressingMode::ZeroPageIndexed, 5, 0),
    Opcode(Instruction::PLA, AddressingMode::Implied, 4, 0),
    Opcode(Instruction::ADC, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::ROR, AddressingMode::Accumulator, 2, 0),
    Opcode(Instruction::None, AddressingMode::None, 0, 0),
    Opcode(Instruction::JMP, AddressingMode::Indirect, 5, 0),
    Opcode(Instruction::ADC, AddressingMode::Absolute, 4, 0),
    Opcode(Instruction::ROR, AddressingMode::Absolute, 6, 0),
    Opcode(Instruction::RRA, AddressingMode::Absolute, 6, 0),

    // 0x70
    Opcode(Instruction::BVS, AddressingMode::Relative, 2, 1),
    Opcode(Instruction::ADC, AddressingMode::IndirectIndexed, 5, 1),
    Opcode(Instruction::JAM, AddressingMode::Implied, 0, 0),
    Opcode(Instruction::RRA, AddressingMode::IndirectIndexed, 8, 0),
    Opcode(Instruction::NOP, AddressingMode::ZeroPageX, 4, 0),
    Opcode(Instruction::ADC, AddressingMode::ZeroPageX, 4, 0),
    Opcode(Instruction::ROR, AddressingMode::ZeroPageX, 6, 0),
    Opcode(Instruction::RRA, AddressingMode::ZeroPageX, 6, 0),
    Opcode(Instruction::SEI, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::ADC, AddressingMode::AbsoluteY, 4, 1),
    Opcode(Instruction::NOP, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::RRA, AddressingMode::AbsoluteY, 7, 0),
    Opcode(Instruction::NOP, AddressingMode::AbsoluteX, 4, 1),
    Opcode(Instruction::ADC, AddressingMode::AbsoluteX, 4, 1),
    Opcode(Instruction::ROR, AddressingMode::AbsoluteX, 7, 0),
    Opcode(Instruction::RRA, AddressingMode::AbsoluteX, 7, 0),

    // 0x80
    Opcode(Instruction::NOP, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::STA, AddressingMode::IndexedIndirect, 6, 0),
    Opcode(Instruction::NOP, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::SAX, AddressingMode::IndexedIndirect, 6, 0),
    Opcode(Instruction::STY, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::STA, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::STX, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::SAX, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::DEY, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::NOP, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::TXA, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::None, AddressingMode::None, 0, 0),
    Opcode(Instruction::STY, AddressingMode::Absolute, 4, 0),
    Opcode(Instruction::STA, AddressingMode::Absolute, 4, 0),
    Opcode(Instruction::STX, AddressingMode::Absolute, 4, 0),
    Opcode(Instruction::SAX, AddressingMode::Absolute, 4, 0),

    // 0x90
    Opcode(Instruction::BCC, AddressingMode::Relative, 2, 1),
    Opcode(Instruction::STA, AddressingMode::IndirectIndexed, 6, 0),
    Opcode(Instruction::JAM, AddressingMode::Implied, 0, 0),
    Opcode(Instruction::None, AddressingMode::None, 0, 0),
    Opcode(Instruction::STY, AddressingMode::ZeroPageX, 4, 0),
    Opcode(Instruction::STA, AddressingMode::ZeroPageX, 4, 0),
    Opcode(Instruction::STX, AddressingMode::ZeroPageY, 4, 0),
    Opcode(Instruction::SAX, AddressingMode::ZeroPageY, 4, 0),
    Opcode(Instruction::TYA, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::STA, AddressingMode::AbsoluteY, 5, 0),
    Opcode(Instruction::TXS, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::None, AddressingMode::None, 0, 0),
    Opcode(Instruction::None, AddressingMode::None, 0, 0),
    Opcode(Instruction::STA, AddressingMode::AbsoluteX, 5, 0),
    Opcode(Instruction::None, AddressingMode::None, 0, 0),
    Opcode(Instruction::None, AddressingMode::None, 0, 0),

    // 0xA0
    Opcode(Instruction::LDY, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::LDA, AddressingMode::IndexedIndirect, 6, 0),
    Opcode(Instruction::LDX, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::LAX, AddressingMode::IndexedIndirect, 6, 0),
    Opcode(Instruction::LDY, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::LDA, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::LDX, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::LAX, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::TAY, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::LDA, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::TAX, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::None, AddressingMode::None, 0, 0),
    Opcode(Instruction::LDY, AddressingMode::Absolute, 4, 0),
    Opcode(Instruction::LDA, AddressingMode::Absolute, 4, 0),
    Opcode(Instruction::LDX, AddressingMode::Absolute, 4, 0),
    Opcode(Instruction::LAX, AddressingMode::Absolute, 4, 0),

    // 0xB0
    Opcode(Instruction::BCS, AddressingMode::Relative, 2, 1),
    Opcode(Instruction::LDA, AddressingMode::IndirectIndexed, 5, 1),
    Opcode(Instruction::JAM, AddressingMode::Implied, 0, 0),
    Opcode(Instruction::LAX, AddressingMode::IndirectIndexed, 5, 1),
    Opcode(Instruction::LDY, AddressingMode::ZeroPageX, 4, 0),
    Opcode(Instruction::LDA, AddressingMode::ZeroPageX, 4, 0),
    Opcode(Instruction::LDX, AddressingMode::ZeroPageY, 4, 0),
    Opcode(Instruction::LAX, AddressingMode::ZeroPageY, 4, 0),
    Opcode(Instruction::CLV, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::LDA, AddressingMode::AbsoluteY, 4, 1),
    Opcode(Instruction::TSX, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::None, AddressingMode::None, 0, 0),
    Opcode(Instruction::LDY, AddressingMode::AbsoluteX, 4, 1),
    Opcode(Instruction::LDA, AddressingMode::AbsoluteX, 4, 1),
    Opcode(Instruction::LDX, AddressingMode::AbsoluteY, 4, 1),
    Opcode(Instruction::LAX, AddressingMode::AbsoluteY, 4, 1),

    // 0xC0
    Opcode(Instruction::CPY, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::CMP, AddressingMode::IndexedIndirect, 6, 0),
    Opcode(Instruction::NOP, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::DCP, AddressingMode::IndexedIndirect, 8, 0),
    Opcode(Instruction::CPY, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::CMP, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::DEC, AddressingMode::ZeroPageIndexed, 5, 0),
    Opcode(Instruction::DCP, AddressingMode::ZeroPageIndexed, 5, 0),
    Opcode(Instruction::INY, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::CMP, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::DEX, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::None, AddressingMode::None, 0, 0),
    Opcode(Instruction::CPY, AddressingMode::Absolute, 4, 0),
    Opcode(Instruction::CMP, AddressingMode::Absolute, 4, 0),
    Opcode(Instruction::DEC, AddressingMode::Absolute, 6, 0),
    Opcode(Instruction::DCP, AddressingMode::Absolute, 6, 0),

    // 0xD0
    Opcode(Instruction::BNE, AddressingMode::Relative, 2, 1),
    Opcode(Instruction::CMP, AddressingMode::IndirectIndexed, 5, 1),
    Opcode(Instruction::JAM, AddressingMode::Implied, 0, 0),
    Opcode(Instruction::DCP, AddressingMode::IndirectIndexed, 8, 0),
    Opcode(Instruction::NOP, AddressingMode::ZeroPageX, 4, 0),
    Opcode(Instruction::CMP, AddressingMode::ZeroPageX, 4, 0),
    Opcode(Instruction::DEC, AddressingMode::ZeroPageX, 6, 0),
    Opcode(Instruction::DCP, AddressingMode::ZeroPageX, 6, 0),
    Opcode(Instruction::CLD, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::CMP, AddressingMode::AbsoluteY, 4, 1),
    Opcode(Instruction::NOP, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::DCP, AddressingMode::AbsoluteY, 7, 0),
    Opcode(Instruction::NOP, AddressingMode::AbsoluteX, 4, 1),
    Opcode(Instruction::CMP, AddressingMode::AbsoluteX, 4, 1),
    Opcode(Instruction::DEC, AddressingMode::AbsoluteX, 7, 0),
    Opcode(Instruction::DCP, AddressingMode::AbsoluteX, 7, 0),

    // 0xE0
    Opcode(Instruction::CPX, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::SBC, AddressingMode::IndexedIndirect, 6, 0),
    Opcode(Instruction::NOP, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::ISB, AddressingMode::IndexedIndirect, 8, 0),
    Opcode(Instruction::CPX, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::SBC, AddressingMode::ZeroPageIndexed, 3, 0),
    Opcode(Instruction::INC, AddressingMode::ZeroPageIndexed, 5, 0),
    Opcode(Instruction::ISB, AddressingMode::ZeroPageIndexed, 5, 0),
    Opcode(Instruction::INX, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::SBC, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::NOP, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::SBC, AddressingMode::Immediate, 2, 0),
    Opcode(Instruction::CPX, AddressingMode::Absolute, 4, 0),
    Opcode(Instruction::SBC, AddressingMode::Absolute, 4, 0),
    Opcode(Instruction::INC, AddressingMode::Absolute, 6, 0),
    Opcode(Instruction::ISB, AddressingMode::Absolute, 6, 0),

    // 0xF0
    Opcode(Instruction::BEQ, AddressingMode::Relative, 2, 1),
    Opcode(Instruction::SBC, AddressingMode::IndirectIndexed, 5, 1),
    Opcode(Instruction::JAM, AddressingMode::Implied, 0, 0),
    Opcode(Instruction::ISB, AddressingMode::IndirectIndexed, 8, 0),
    Opcode(Instruction::NOP, AddressingMode::ZeroPageX, 4, 0),
    Opcode(Instruction::SBC, AddressingMode::ZeroPageX, 4, 0),
    Opcode(Instruction::INC, AddressingMode::ZeroPageX, 6, 0),
    Opcode(Instruction::ISB, AddressingMode::ZeroPageX, 6, 0),
    Opcode(Instruction::SED, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::SBC, AddressingMode::AbsoluteY, 4, 1),
    Opcode(Instruction::NOP, AddressingMode::Implied, 2, 0),
    Opcode(Instruction::ISB, AddressingMode::AbsoluteY, 7, 0),
    Opcode(Instruction::NOP, AddressingMode::AbsoluteX, 4, 1),
    Opcode(Instruction::SBC, AddressingMode::AbsoluteX, 4, 1),
    Opcode(Instruction::INC, AddressingMode::AbsoluteX, 7, 0),
    Opcode(Instruction::ISB, AddressingMode::AbsoluteX, 7, 0),
];
pub struct CPU6507 {
    bus: Box<dyn Bus>,

    // Main registers
    pub a: u8,  // Accumulator
    pub x: u8,  // X Index
    pub y: u8,  // Y Index

    // Status register flags
    c: bool,  // Carry
    z: bool,  // Zero
    i: bool,  // Interrupt
    d: bool,  // Decimal mode
    b: bool,  // Software interrupt (BRK)
    u: bool,  // Unused flag
    v: bool,  // Overflow
    s: bool,  // Sign

    // Program counter
    pub pc: u16,

    // Stack pointer
    sp: u8,

    // Total number of cycles executed
    cycles: u64,

    current_instruction: Option<Instruction>,
    current_addr: u16,
    current_addr_mode: AddressingMode,
    current_cycles: u64,
}

impl Bus for CPU6507 {
    fn read(&mut self, addr: u16) -> u8 {
        // The 6507 only had 13 address lines connected.
        self.bus.read(addr & 0x1fff)
    }

    fn write(&mut self, addr: u16, val: u8) {
        // The 6507 only had 13 address lines connected.
        self.bus.write(addr & 0x1fff, val);
    }
}

impl CPU6507 {
    pub fn new(bus: Box<dyn Bus>) -> Self {
        Self {
            bus: bus,

            a: 0,
            x: 0,
            y: 0,

            c: false,
            z: false,
            i: false,
            d: false,
            b: false,
            u: false,
            v: false,
            s: false,

            pc: 0x0000,

            sp: STACK_INIT,

            cycles: 0,

            current_instruction: None,
            current_addr: 0x0000,
            current_addr_mode: AddressingMode::Accumulator,
            current_cycles: 0,
        }
    }

    pub fn reset(&mut self) {
        let lo = self.read(0xFFFC) as u16;
        let hi = self.read(0xFFFD) as u16;
        let addr = (hi << 8) | lo;
        self.pc = addr;
        info!("PC: 0x{:04X}", self.pc);

        self.set_flags(0x24);

        self.sp = STACK_INIT;
        self.a = 0;
        self.x = 0;
        self.y = 0;

        self.cycles = 0;
    }

    fn flags(&self) -> u8 {
           (self.c as u8)
        | ((self.z as u8) << 1)
        | ((self.i as u8) << 2)
        | ((self.d as u8) << 3)
        | ((self.b as u8) << 4)
        | ((self.u as u8) << 5)
        | ((self.v as u8) << 6)
        | ((self.s as u8) << 7)
    }

    fn set_flags(&mut self, val: u8) {
        self.c = val & 0x01 == 1;
        self.z = (val >> 1 & 0x01) == 1;
        self.i = (val >> 2 & 0x01) == 1;
        self.d = (val >> 3 & 0x01) == 1;
        self.b = (val >> 4 & 0x01) == 1;
        self.u = (val >> 5 & 0x01) == 1;
        self.v = (val >> 6 & 0x01) == 1;
        self.s = (val >> 7 & 0x01) == 1;
    }

    fn debug(&mut self, op: &Opcode) {
        let Opcode(ref inst, ref addr_mode, _, _) = *op;

        let raw_bytes = addr_mode.get_bytes(self);

        let bytes = raw_bytes.iter()
            .map(|arg| String::from(format!("{:02X}", arg)))
            .collect::<Vec<_>>()
            .join(" ");

        println!("{:04X}  {:8}  {:32?} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}",
                 self.pc,
                 bytes,
                 inst,
                 self.a,
                 self.x,
                 self.y,
                 self.flags(),
                 self.sp);
    }

    fn stack_push8(&mut self, val: u8) {
        // The stack page exists from 0x0080 to 0x00FF
        let addr = 0x0000 | (self.sp as u16);
        self.write(addr, val);

        let n = self.sp.wrapping_sub(1);
        self.sp = n;
    }

    fn stack_pop8(&mut self) -> u8 {
        let n = self.sp.wrapping_add(1);
        self.sp = n;

        // The stack page exists from 0x0080 to 0x00FF
        let addr = 0x0000 | (self.sp as u16);
        let val = self.read(addr);

        val
    }

    fn stack_push16(&mut self, val: u16) {
        let hi = (val >> 8) as u8;
        self.stack_push8(hi);

        let lo = (val & 0x00ff) as u8;
        self.stack_push8(lo);
    }

    fn stack_pop16(&mut self) -> u16 {
        let lo = self.stack_pop8() as u16;
        let hi = self.stack_pop8() as u16;
        (hi << 8) | lo
    }

    fn update_sz(&mut self, val: u8) {
        self.s = val & 0x80 != 0;
        self.z = val == 0;
    }

    fn add_branch_cycles(&mut self, pc: u16, addr: u16) {
        self.current_cycles += 1;
        self.cycles += 1;

        // It costs an extra cycle to branch to a different page.
        if (pc & 0xff00) != (addr & 0xff00) {
            self.current_cycles += 1;
            self.cycles += 1;
        }
    }

    fn fetch_and_decode(&mut self) -> u64 {
        let opcode = self.read(self.pc);
        let op = &OPCODES[opcode as usize];

        if *CPU6507_DEBUG {
            self.debug(&op);
        }

        let &Opcode(ref inst, ref addr_mode, cycles, extra_cycles) = op;
        let (addr, page_crossed) = addr_mode.get_data(self);

        self.pc += addr_mode.n_bytes() as u16;
        self.current_instruction = Some(*inst);
        self.current_addr = addr;
        self.current_addr_mode = *addr_mode;

        let mut new_cycles = cycles;
        if page_crossed {
            new_cycles += extra_cycles;
        }

        new_cycles
    }

    fn execute(&mut self) {
        if let Some(inst) = self.current_instruction {
            let addr = self.current_addr;
            let addr_mode = self.current_addr_mode;

            match inst {
                Instruction::ADC => self.adc(addr),
                Instruction::ANC => self.anc(addr),
                Instruction::AND => self.and(addr),
                Instruction::ASL => self.asl(addr, addr_mode),
                Instruction::BCC => self.bcc(addr),
                Instruction::BCS => self.bcs(addr),
                Instruction::BEQ => self.beq(addr),
                Instruction::BIT => self.bit(addr),
                Instruction::BMI => self.bmi(addr),
                Instruction::BNE => self.bne(addr),
                Instruction::BPL => self.bpl(addr),
                Instruction::BRK => self.brk(),
                Instruction::BVC => self.bvc(addr),
                Instruction::BVS => self.bvs(addr),
                Instruction::CLC => self.clc(),
                Instruction::CLD => self.cld(),
                Instruction::CLI => self.cli(),
                Instruction::CLV => self.clv(),
                Instruction::CMP => self.cmp(addr),
                Instruction::CPX => self.cpx(addr),
                Instruction::CPY => self.cpy(addr),
                Instruction::DCP => self.dcp(addr),
                Instruction::DEC => self.dec(addr),
                Instruction::DEX => self.dex(),
                Instruction::DEY => self.dey(),
                Instruction::EOR => self.eor(addr),
                Instruction::INC => self.inc(addr),
                Instruction::INX => self.inx(),
                Instruction::INY => self.iny(),
                Instruction::ISB => self.isb(addr),
                Instruction::JAM => self.jam(),
                Instruction::JMP => self.jmp(addr),
                Instruction::JSR => self.jsr(addr),
                Instruction::LAX => self.lax(addr),
                Instruction::LDA => self.lda(addr),
                Instruction::LDX => self.ldx(addr),
                Instruction::LDY => self.ldy(addr),
                Instruction::LSR => self.lsr(addr, addr_mode),
                Instruction::NOP => self.nop(),
                Instruction::ORA => self.ora(addr),
                Instruction::PHA => self.pha(),
                Instruction::PHP => self.php(),
                Instruction::PLA => self.pla(),
                Instruction::PLP => self.plp(),
                Instruction::RLA => self.rla(addr, addr_mode),
                Instruction::ROL => self.rol(addr, addr_mode),
                Instruction::ROR => self.ror(addr, addr_mode),
                Instruction::RRA => self.rra(addr, addr_mode),
                Instruction::RTI => self.rti(),
                Instruction::RTS => self.rts(),
                Instruction::SAX => self.sax(addr),
                Instruction::SBC => self.sbc(addr),
                Instruction::SEC => self.sec(),
                Instruction::SED => self.sed(),
                Instruction::SEI => self.sei(),
                Instruction::SLO => self.slo(addr, addr_mode),
                Instruction::SRE => self.sre(addr, addr_mode),
                Instruction::STA => self.sta(addr),
                Instruction::STX => self.stx(addr),
                Instruction::STY => self.sty(addr),
                Instruction::TAX => self.tax(),
                Instruction::TAY => self.tay(),
                Instruction::TSX => self.tsx(),
                Instruction::TXA => self.txa(),
                Instruction::TXS => self.txs(),
                Instruction::TYA => self.tya(),
                _ => panic!("unsupported instruction {:?}", inst),
            }

            self.current_instruction = None;
        }
    }

    pub fn step(&mut self) -> u64 {
        let start_cycles = self.cycles;
        self.cycles += self.fetch_and_decode();
        self.execute();
        self.cycles - start_cycles
    }

    pub fn clock(&mut self) {
        if self.current_cycles == 0 {
            self.current_cycles += self.fetch_and_decode();
        }

        self.current_cycles -= 1;
        if self.current_cycles == 0 {
            self.execute();
        }
    }

    //
    // Legal instructions
    //

    fn adc(&mut self, addr: u16) {
        let val = self.read(addr);

        if self.d {
            let mut lo = (self.a as u16 & 0x0f) + (val as u16 & 0x0f) + (self.c as u16);
            let mut hi = (self.a as u16 & 0xf0) + (val as u16 & 0xf0);

            // In BCD, values 0x0A to 0x0F are invalid, so we add 1 to the high nybble for the
            // carry, and the low nybble has to skip 6 values for A-F.
            if lo > 0x09 {
                hi += 0x10;
                lo += 0x06;
            }

            self.s = (hi & 0x80) != 0;
            self.z = ((lo + hi) & 0xff) != 0;
            self.v = ((self.a ^ val) & 0x80 == 0) && ((self.a ^ hi as u8) & 0x80 != 0);

            // 0xA0 to 0xF0 are invalid for the high nybble, so we need to skip 6 values of the
            // high nybble.
            if hi > 0x90 {
                hi += 0x60;
            }

            debug!("ADC  A:{:02X}, val:{:02X}, C:{:02X}: res:{:02X}",
                   self.a, val, self.c as u8, (hi & 0xf0) | (lo & 0x0f));

            //self.c = (hi & 0xff00) != 0;
            self.a = (lo & 0x0f) as u8 | (hi & 0xf0) as u8;
        } else {
            let n = (self.a as u16) + (val as u16) + (self.c as u16);
            let a = (n & 0x00ff) as u8;

            self.update_sz(a);
            self.c = n > 0xff;

            // The first condition checks if the sign of the accumulator and the
            // the sign of value that we're adding are the same.
            //
            // The second condition checks if the result of the addition has a
            // different sign to either of the values we added together.
            self.v = ((self.a ^ val) & 0x80 == 0) && ((self.a ^ a) & 0x80 != 0);

            self.a = a;
        }
    }

    fn and(&mut self, addr: u16) {
        let val = self.read(addr);
        self.a &= val;
        let a = self.a;
        self.update_sz(a);
    }

    fn asl(&mut self, addr: u16, addr_mode: AddressingMode) {
        let val = match addr_mode {
            AddressingMode::Accumulator => self.a,
            _ => self.read(addr),
        };

        self.c = val & 0x80 != 0;
        let n = (val << 1) & 0xff;

        match addr_mode {
            AddressingMode::Accumulator => { self.a = n; },
            _ => { self.write(addr, n); }
        };

        self.update_sz(n);
    }

    fn bcc(&mut self, addr: u16) {
        if !self.c {
            let pc = self.pc;
            self.add_branch_cycles(pc, addr);
            self.pc = addr;
        }
    }

    fn bcs(&mut self, addr: u16) {
        if self.c {
            let pc = self.pc;
            self.add_branch_cycles(pc, addr);
            self.pc = addr;
        }
    }

    fn beq(&mut self, addr: u16) {
        if self.z {
            let pc = self.pc;
            self.add_branch_cycles(pc, addr);
            self.pc = addr;
        }
    }

    fn bit(&mut self, addr: u16) {
        let val = self.read(addr);
        self.s = val & 0x80 != 0;
        self.v = (val >> 0x06 & 0x01) == 1;
        let f = self.a & val;
        self.z = f == 0;
    }

    fn bmi(&mut self, addr: u16) {
        if self.s {
            let pc = self.pc;
            self.add_branch_cycles(pc, addr);
            self.pc = addr;
        }
    }

    fn bne(&mut self, addr: u16) {
        if !self.z {
            let pc = self.pc;
            self.add_branch_cycles(pc, addr);
            self.pc = addr;
        }
    }

    fn bpl(&mut self, addr: u16) {
        if !self.s {
            let pc = self.pc;
            self.add_branch_cycles(pc, addr);
            self.pc = addr;
        }
    }

    fn brk(&mut self) {
        let pc = self.pc + 1;
        self.stack_push16(pc);

        self.b = true;

        let flags = self.flags() | 0x10;
        self.stack_push8(flags);

        self.i = true;

        let lo = self.read(0xFFFE) as u16;
        let hi = self.read(0xFFFF) as u16;
        let pc = (hi << 8) | lo;
        self.pc = pc;
    }

    fn bvc(&mut self, addr: u16) {
        if !self.v {
            let pc = self.pc;
            self.add_branch_cycles(pc, addr);
            self.pc = addr;
        }
    }

    fn bvs(&mut self, addr: u16) {
        if self.v {
            let pc = self.pc;
            self.add_branch_cycles(pc, addr);
            self.pc = addr;
        }
    }

    fn clc(&mut self) {
        self.c = false;
    }

    fn cld(&mut self) {
        self.d = false;
    }

    fn cli(&mut self) {
        self.i = false;
    }

    fn clv(&mut self) {
        self.v = false;
    }

    fn cmp(&mut self, addr: u16) {
        let val = self.read(addr);
        let n = self.a.wrapping_sub(val);
        self.c = self.a >= val;
        self.update_sz(n);
    }

    fn cpx(&mut self, addr: u16) {
        let val = self.read(addr);
        let n = self.x.wrapping_sub(val);
        self.update_sz(n);
        self.c = self.x >= val;
    }

    fn cpy(&mut self, addr: u16) {
        let val = self.read(addr);
        let n = self.y.wrapping_sub(val);
        self.update_sz(n);
        self.c = self.y >= val;
    }

    fn dec(&mut self, addr: u16) {
        let val = self.read(addr);
        let n = val.wrapping_sub(1);
        self.update_sz(n);
        self.write(addr, n);
    }

    fn dex(&mut self) {
        let n = self.x.wrapping_sub(1);
        self.x = n;
        self.update_sz(n);
    }

    fn dey(&mut self) {
        let n = self.y.wrapping_sub(1);
        self.y = n;
        self.update_sz(n);
    }

    fn eor(&mut self, addr: u16) {
        let val = self.read(addr);
        let val = val ^ self.a;
        self.a = val;
        self.update_sz(val);
    }

    fn inc(&mut self, addr: u16) {
        let val = self.read(addr);
        let n = val.wrapping_add(1);
        self.write(addr, n);
        self.update_sz(n);
    }

    fn inx(&mut self) {
        let n = self.x.wrapping_add(1);
        self.x = n;
        self.update_sz(n);
    }

    fn iny(&mut self) {
        let n = self.y.wrapping_add(1);
        self.y = n;
        self.update_sz(n);
    }

    fn jmp(&mut self, addr: u16) {
        self.pc = addr;
    }

    fn jsr(&mut self, addr: u16) {
        let retaddr = self.pc - 1;
        self.stack_push16(retaddr);
        self.pc = addr;
    }

    fn lda(&mut self, addr: u16) {
        let val = self.read(addr);
        self.a = val;
        self.update_sz(val);
    }

    fn ldx(&mut self, addr: u16) {
        let val = self.read(addr);
        self.x = val;
        self.update_sz(val);
    }

    fn ldy(&mut self, addr: u16) {
        let val = self.read(addr);
        self.y = val;
        self.update_sz(val);
    }

    fn lsr(&mut self, addr: u16, addr_mode: AddressingMode) {
        let val = match addr_mode {
            AddressingMode::Accumulator => self.a,
            _ => self.read(addr),
        };

        self.c = val & 0x01 == 1;
        let n = val >> 1;
        self.update_sz(n);

        match addr_mode {
            AddressingMode::Accumulator => { self.a = n; },
            _ => { self.write(addr, n); }
        };
    }

    fn nop(&self) { }

    fn ora(&mut self, addr: u16) {
        let val = self.read(addr);
        let na = self.a | val;
        self.a = na;
        self.update_sz(na);
    }

    fn pha(&mut self) {
        let a = self.a;
        self.stack_push8(a);
    }

    fn php(&mut self) {
        // https://wiki.nesdev.com/w/index.php/CPU_status_flag_behavior
        // According to the above link, the PHP instruction sets bits 4 and 5 on
        // the value it pushes onto the stack.
        // The PLP call later will ignore these bits.
        let flags = self.flags() | 0x10;
        self.stack_push8(flags);
    }

    fn pla(&mut self) {
        let rv = self.stack_pop8();
        self.a = rv;
        self.update_sz(rv);
    }

    fn plp(&mut self) {
        let p = self.stack_pop8() & 0xef | 0x20;
        self.set_flags(p);
    }

    fn rol(&mut self, addr: u16, addr_mode: AddressingMode) {
        let val = match addr_mode {
            AddressingMode::Accumulator => self.a,
            _ => self.read(addr),
        };

        let n = (val << 1) | (self.c as u8);
        self.c = val & 0x80 != 0;
        self.update_sz(n);

        match addr_mode {
            AddressingMode::Accumulator => { self.a = n; },
            _ => { self.write(addr, n); }
        };
    }

    fn ror(&mut self, addr: u16, addr_mode: AddressingMode) {
        let val = match addr_mode {
            AddressingMode::Accumulator => self.a,
            _ => self.read(addr),
        };

        let n = (val >> 1) | ((self.c as u8) << 7);
        self.c = val & 0x01 == 1;
        self.update_sz(n);

        match addr_mode {
            AddressingMode::Accumulator => { self.a = n; },
            _ => { self.write(addr, n); }
        };
    }

    fn rti(&mut self) {
        let flags = self.stack_pop8() & 0xef | 0x20;
        self.set_flags(flags);

        let retaddr = self.stack_pop16();
        self.pc = retaddr;
    }

    fn rts(&mut self) {
        let retaddr = self.stack_pop16();
        self.pc = retaddr + 1;
    }

    fn sbc(&mut self, addr: u16) {
        let val = self.read(addr);

        if self.d {
            // http://www.6502.org/tutorials/decimal_mode.html
            let mut temp = (self.a as i16) - (val as i16) - (!self.c as i16);
            let mut lo = ((self.a as i16) & 0x0f) - ((val as i16) & 0x0f) - (!self.c as i16);

            if temp < 0 {
                temp -= 0x60;
            }

            if lo < 0 {
                temp -= 0x06;
            }

            debug!("SBC  {:02X} - {:02X} - {:02X} = {:04X}", self.a, val, !self.c as u8, temp);

            let a = (temp & 0xff) as u8;
            self.update_sz(a);
            self.v = ((self.a ^ val) & 0x80 == 0) && ((self.a ^ a) & 0x80 != 0);
            self.c = temp >= 0;
            self.a = a;
        } else {
            let val = ! val;
            let n = (self.a as u16) + (val as u16) + (self.c as u16);
            let a = (n & 0x00ff) as u8;

            self.update_sz(a);
            self.c = n > 0xff;

            // The first condition checks if the sign of the accumulator and the
            // the sign of value that we're adding are the same.
            //
            // The second condition checks if the result of the addition has a
            // different sign to either of the values we added together.
            self.v = ((self.a ^ val) & 0x80 == 0) && ((self.a ^ a) & 0x80 != 0);

            self.a = a;
        }
    }

    fn sec(&mut self) {
        self.c = true;
    }

    fn sed(&mut self) {
        self.d = true;
    }

    fn sei(&mut self) {
        self.i = true;
    }

    fn sta(&mut self, addr: u16) {
        self.write(addr, self.a);
    }

    fn stx(&mut self, addr: u16) {
        self.write(addr, self.x);
    }

    fn sty(&mut self, addr: u16) {
        self.write(addr, self.y);
    }

    fn tax(&mut self) {
        let n = self.a;
        self.x = n;
        self.update_sz(n);
    }

    fn tay(&mut self) {
        let n = self.a;
        self.y = n;
        self.update_sz(n);
    }

    fn tsx(&mut self) {
        let s = self.sp;
        self.update_sz(s);
        self.x = s;
    }

    fn txa(&mut self) {
        let n = self.x;
        self.a = n;
        self.update_sz(n);
    }

    fn txs(&mut self) {
        self.sp = self.x;
    }

    fn tya(&mut self) {
        let n = self.y;
        self.a = n;
        self.update_sz(n);
    }

    //
    // Illegal instructions
    //

    fn anc(&mut self, addr: u16) {
        let val = self.read(addr);
        let a = self.a & val;
        self.a = a;
        self.update_sz(a);
        self.c = (a as i8) < 0;
    }

    fn lax(&mut self, addr: u16) {
        let val = self.read(addr);
        self.a = val;
        self.x = val;
        self.update_sz(val);
    }

    fn sax(&mut self, addr: u16) {
        let val = self.x & self.a;
        self.write(addr, val);
    }

    fn dcp(&mut self, addr: u16) {
        // Copied from dec
        let val = self.read(addr);
        let n = val.wrapping_sub(1);
        self.update_sz(n);
        self.write(addr, n);

        // Copied from cmp
        let n = self.a.wrapping_sub(n);
        self.c = self.a >= n;
        self.update_sz(n);
    }

    fn isb(&mut self, addr: u16) {
        // Copied from inc
        let val = self.read(addr);
        let n = val.wrapping_add(1);
        self.write(addr, n);
        self.update_sz(n);

        // Copied from sbc
        let val = n;
        let n: i16 = (self.a as i16)
            .wrapping_sub(val as i16)
            .wrapping_sub(1 - self.c as i16);

        let a = n as u8;
        self.update_sz(a);
        self.v = ((self.a ^ val) & 0x80 > 0) && ((self.a ^ n as u8) & 0x80 > 0);
        self.a = a;
        self.c = n >= 0;
    }

    fn slo(&mut self, addr: u16, addr_mode: AddressingMode) {
        // Copied from asl
        let val = self.read(addr);
        self.c = val & 0x80 != 0;
        let n = (val << 1) & 0xff;

        match addr_mode {
            AddressingMode::Accumulator => { self.a = n; },
            _ => { self.write(addr, n); }
        };

        self.update_sz(n);

        // Copied from ora
        let val = n;
        let na = self.a | val;
        self.a = na;
        self.update_sz(na);
    }

    fn rla(&mut self, addr: u16, addr_mode: AddressingMode) {
        // Copied from rol
        let val = self.read(addr);
        let c = self.c;
        self.c = val & 0x80 != 0;
        let n = (val << 1) | (c as u8);
        self.update_sz(n);

        match addr_mode {
            AddressingMode::Accumulator => { self.a = n; },
            _ => { self.write(addr, n); }
        };

        // Copied from and
        let val = n;
        self.a &= val;
        let a = self.a;
        self.update_sz(a);
    }

    fn sre(&mut self, addr: u16, addr_mode: AddressingMode) {
        // Copied from lsr
        let val = self.read(addr);
        self.c = val & 0x01 == 1;
        let n = val >> 1;
        self.update_sz(n);

        match addr_mode {
            AddressingMode::Accumulator => { self.a = n; },
            _ => { self.write(addr, n); }
        };

        // Copied from eor
        let val = n;
        let val = val ^ self.a;
        self.a = val;
        self.update_sz(val);
    }

    fn rra(&mut self, addr: u16, addr_mode: AddressingMode) {
        // Copied from ror
        let val = self.read(addr);
        let c = self.c;
        self.c = val & 0x01 == 1;
        let n = (val >> 1) | ((c as u8) << 7);
        self.update_sz(n);

        match addr_mode {
            AddressingMode::Accumulator => { self.a = n; },
            _ => { self.write(addr, n); }
        };

        // Copied from adc
        let val = n;
        let n = (val as u16) + (self.a as u16) + (self.c as u16);
        let a = (n & 0xff) as u8;
        self.update_sz(a);
        self.c = n > 0xff;
        self.v = ((self.a ^ val) & 0x80 == 0) && ((self.a ^ n as u8) & 0x80 > 0);
        self.a = a;
    }

    fn jam(&mut self) {
        process::exit(0);
    }
}
