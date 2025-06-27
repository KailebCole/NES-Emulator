// The NES's 2A03 is a modified version of the 6502 chip. 
// As with any CPU, the goal of this module is to execute the main program instructions.

// Memory Map:
// RAM:             [0x0000 ... 0x2000]
// IO Registers:    [0x2000 ... 0x4020]
// Expansion ROM:   [0x4020 ... 0x6000]
// Save RAM:        [0x6000 ... 0x8000]
// Program ROM:     [0x8000 ... 0xFFFF]

// Registers:
// Program Counter:     Next Instruction Address
// Stack Pointer:       [0x100 ... 0x1FF] Stack Address, top to bottom
// Accumulator:         Stores the result of arithmetic, logic, and memory operations
// Index X:             General Register
// Index Y:             General Register
// Processor Status:    Represents 7 status flags

use std::collections::HashMap;
use crate::{bus, opcodes::{self, OPCode}};

const STACK: u16 = 0x0100;
const STACK_RESET: u8 = 0xFD;

pub struct CPU {
    pub register_a: u8,
    pub register_x: u8,
    pub register_y: u8,
    pub register_sp: u8,
    pub register_pc: u16,
    pub flags: Flags,
    pub bus: bus::Bus,
    instructions_executed: u128,
}

#[derive(Clone)]
pub struct Flags {
    pub bits: u8
    /* 
    N V U B D I Z C
    | |   | | | | +---- Carry
    | |   | | | +------ Zero
    | |   | | +-------- Interrupt Disable
    | |   | +---------- Decimal (Not Used)
    | |   +------------ Break
    | +---------------- Overflow
    +------------------ Negative
    */
}

#[derive(Debug)]
pub enum AddressingMode {
    Immediate,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteX,
    AbsoluteY,
    IndirectX,
    IndirectY,
    NoneAddressing,
}

pub trait Mem {
    // Read the data byte at a spectific adddress
    fn mem_read(&self, addr: u16) -> u8;

    // Write a data byte a specific memory address
    fn mem_write(&mut self, addr: u16, data: u8);

    // Read two data bytes in little endian format at address
    fn mem_read_16(&self, addr: u16) -> u16 {
        let lo = self.mem_read(addr) as u16;
        let hi = self.mem_read(addr + 1) as u16;
        (hi << 8) | (lo as u16)
    }

    // Write two data bytes in little endian format at address
    fn mem_write_16(&mut self, addr: u16, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xFF) as u8;
        self.mem_write(addr, lo);
        self.mem_write(addr + 1, hi);
    }
}

impl Mem for CPU {
    fn mem_read(&self, addr: u16) -> u8 { 
        return self.bus.mem_read(addr)
    }

    fn mem_write(&mut self, addr: u16, data: u8) { 
        self.bus.mem_write(addr, data);
    }

    fn mem_read_16(&self, addr: u16) -> u16 {
        return self.bus.mem_read_16(addr)
    }

    fn mem_write_16(&mut self, addr: u16, data: u16) {
        self.bus.mem_write_16(addr, data);
    }
}

impl CPU {
    // Initiate the CPU
    pub fn new(bus: bus::Bus) -> Self {
        CPU {
            register_a: 0,
            register_x: 0,
            register_y: 0,
            register_sp: STACK_RESET,
            register_pc: 0,
            flags: Flags::new(),
            bus: bus,
            instructions_executed: 0,
        }
    }

    // Load program into memory starting at PROM location 0x8000
    pub fn load(&mut self, program: Vec<u8>) {
        for i in 0..(program.len() as u16) {
            self.mem_write(0x0600 + 1, program[i as usize]);
        }
        self.mem_write_16(0xFFFC, 0x0600);
    }

    // Reset the Emulator to initial state and reset address
    pub fn reset(&mut self) {
        self.register_a = 0;
        self.register_x = 0;
        self.register_y = 0;
        self.register_sp = STACK_RESET;
        self.flags.bits = 0x24;
        self.instructions_executed = 0;

        self.register_pc = self.mem_read_16(0xFFFC)
    }

    pub fn run(&mut self) {
        self.run_with_callback(|_| {});
    }

    // Decode and execute program file
    pub fn run_with_callback<F>(&mut self, mut callback: F) where F: FnMut(&mut CPU), {
        let ref opcodes: HashMap<u8, &'static opcodes::OPCode> = *opcodes::OPCodes_MAP;

        loop {
            callback(self);

            // instruction limit safeguard
            self.instructions_executed += 1;
            if self.instructions_executed > 5_000_000 {
                println!("Instruction limit reached.");
                std::process::exit(1);
            }

            let code = self.mem_read(self.register_pc);
            self.register_pc += 1;
            let program_counter_state = self.register_pc;

            let opcode = opcodes.get(&code).expect(&format!("OPCode {:x} is not recognized", code));
            
            // Check the opcode with each opcode case
            match code {
                /* RET */ 0x00 =>                                                   return,
                /* ADC */ 0x69 | 0x65 | 0x75 | 0x6d | 0x7d | 0x79 | 0x61 | 0x71 =>  {self.adc(&opcode.mode)},
                /* AND */ 0x29 | 0x25 | 0x35 | 0x2d | 0x3d | 0x39 | 0x21 | 0x31 =>  {self.and(&opcode.mode)},
                /* ASL */ 0x0a =>                                                   {self.asl_a()},
                /* ASL */ 0x06 | 0x16 | 0x0e | 0x1e =>                              {self.asl(&opcode.mode);},
                /* BCC */ 0x90 =>                                                   {self.bcc()},
                /* BCS */ 0xb0 =>                                                   {self.bcs()},
                /* BEQ */ 0xf0 =>                                                   {self.beq()},
                /* BIT */ 0x24 | 0x2c =>                                            {self.bit(&opcode.mode)},
                /* BMI */ 0x30 =>                                                   {self.bmi()},
                /* BNE */ 0xd0 =>                                                   {self.bne()},
                /* BPL */ 0x10 =>                                                   {self.bpl()},
                /* BVC */ 0x50 =>                                                   {self.bvc()},
                /* BVS */ 0x70 =>                                                   {self.bvs()},
                /* CLC */ 0x18 =>                                                   {self.clc()},
                /* CLD */ 0xd8 =>                                                   {self.cld()},
                /* CLI */ 0x58 =>                                                   {self.cli()},
                /* CLV */ 0xb8 =>                                                   {self.clv()},
                /* CMP */ 0xc9 | 0xc5 | 0xd5 | 0xcd | 0xdd | 0xd9 | 0xc1 | 0xd1 =>  {self.cmp(&opcode.mode)},
                /* CPX */ 0xe0 | 0xe4 | 0xec =>                                     {self.cpx(&opcode.mode)},
                /* CPY */ 0xc0 | 0xc4 | 0xcc =>                                     {self.cpy(&opcode.mode)},
                /* DEC */ 0xc6 | 0xd6 | 0xce | 0xde =>                              {self.dec(&opcode.mode)},
                /* DEX */ 0xca =>                                                   {self.dex()},
                /* DEY */ 0x88 =>                                                   {self.dey()},
                /* EOR */ 0x49 | 0x45 | 0x55 | 0x4d | 0x5d | 0x59 | 0x41 | 0x51 =>  {self.eor(&opcode.mode)},
                /* INC */ 0xe6 | 0xf6 | 0xee | 0xfe =>                              {self.inc(&opcode.mode);},
                /* INX */ 0xe8 =>                                                   {self.inx()},
                /* INY */ 0xc8 =>                                                   {self.iny()},
                /* JMP */ 0x4c =>                                                   {self.jmp_abs()},
                /* JMP */ 0x6c =>                                                   {self.jmp_ind()},
                /* JSR */ 0x20 =>                                                   {self.jsr()},
                /* LDA */ 0xa9 | 0xa5 | 0xb5 | 0xad | 0xbd | 0xb9 | 0xa1 | 0xb1 =>  {self.lda(&opcode.mode)},
                /* LDX */ 0xa2 | 0xa6 | 0xb6 | 0xae | 0xbe =>                       {self.ldx(&opcode.mode)},
                /* LDY */ 0xa0 | 0xa4 | 0xb4 | 0xac | 0xbc =>                       {self.ldy(&opcode.mode)},
                /* LSR */ 0x4a =>                                                   {self.lsr_a()},
                /* LSR */ 0x46 | 0x56 | 0x4e | 0x5e =>                              {self.lsr(&opcode.mode);},
                /* NOP */ 0xea =>                                                   {self.nop()},
                /* ORA */ 0x09 | 0x05 | 0x15 | 0x0d | 0x1d | 0x19 | 0x01 | 0x11 =>  {self.ora(&opcode.mode)},
                /* PHA */ 0x48 =>                                                   {self.pha()},
                /* PHP */ 0x08 =>                                                   {self.php()},
                /* PLA */ 0x68 =>                                                   {self.pla()},
                /* PLP */ 0x28 =>                                                   {self.plp()},
                /* ROL */ 0x2a =>                                                   {self.rol_a()},
                /* ROL */ 0x26 | 0x36 | 0x2e | 0x3e =>                              {self.rol(&opcode.mode);},
                /* ROR */ 0x6a =>                                                   {self.ror_a()},
                /* ROR */ 0x66 | 0x76 | 0x6e | 0x7e =>                              {self.ror(&opcode.mode);},
                /* RTI */ 0x40 =>                                                   {self.rti()},
                /* RTS */ 0x60 =>                                                   {self.rts()},
                /* SBC */ 0xe9 | 0xe5 | 0xf5 | 0xed | 0xfd | 0xf9 | 0xe1 | 0xf1 =>  {self.sbc(&opcode.mode)},
                /* SEC */ 0x38 =>                                                   {self.sec()},
                /* SED */ 0xf8 =>                                                   {self.sed()},
                /* SEI */ 0x78 =>                                                   {self.sei()},
                /* STA */ 0x85 | 0x95 | 0x8d | 0x9d | 0x99 | 0x81 | 0x91 =>         {self.sta(&opcode.mode)},
                /* STX */ 0x86 | 0x96 | 0x8e =>                                     {self.stx(&opcode.mode)},
                /* STY */ 0x84 | 0x94 | 0x8c =>                                     {self.sty(&opcode.mode)},
                /* TAX */ 0xAA =>                                                   {self.tax()},
                /* TAY */ 0xa8 =>                                                   {self.tay()},
                /* TSX */ 0xba =>                                                   {self.tsx()},
                /* TXA */ 0x8a =>                                                   {self.txa()},
                /* TXS */ 0x9a =>                                                   {self.txs()},
                /* TYA */ 0x98 =>                                                   {self.tya()},

                /* Unofficial */
                /* AHX Absolute Y */ 0x9f =>                                        {self.uahx_ay()},
                /* AHX  Indirect Y */ 0x93 =>                                       {self.uahx_iy()},
                /* ALR */ 0x4b =>                                                   {self.ualr(&opcode.mode)},
                /* ANC */ 0x0b | 0x2b =>                                            {self.uanc(&opcode.mode)},
                /* ARR */ 0x6B =>                                                   {self.uarr(&opcode.mode)},
                /* AXS */ 0xCB =>                                                   {self.uaxs(&opcode.mode)},
                /* DCP */ 0xc7 | 0xd7 | 0xCF | 0xdF | 0xdb | 0xd3 | 0xc3 =>         {self.udcp(&opcode.mode)},
                /* ISB */ 0xe7 | 0xf7 | 0xef | 0xff | 0xfb | 0xe3 | 0xf3 =>         {self.uisb(&opcode.mode)},
                /* LAS */ 0xbb =>                                                   {self.ulas(&opcode.mode)},
                /* LAX */ 0xa7 | 0xb7 | 0xaf | 0xbf | 0xa3 | 0xb3 =>                {self.ulax(&opcode.mode)},
                /* LXA */ 0xab =>                                                   {self.ulxa(&opcode.mode)},
                /* RLA */ 0x27 | 0x37 | 0x2F | 0x3F | 0x3b | 0x33 | 0x23 =>         {self.urla(&opcode.mode)},
                /* RRA */ 0x67 | 0x77 | 0x6f | 0x7f | 0x7b | 0x63 | 0x73 =>         {self.urra(&opcode.mode)},
                /* SAX */ 0x87 | 0x97 | 0x8f | 0x83 =>                              {self.usax(&opcode.mode)},
                /* SBC */ 0xeb =>                                                   {self.usbc(&opcode.mode)},
                /* SHX */ 0x9e =>                                                   {self.ushx()},
                /* SHY */ 0x9c =>                                                   {self.ushy()},
                /* SKB */ 0x80 | 0x82 | 0x89 | 0xc2 | 0xe2 =>                       {self.uskb()},
                /* SLO */ 0x07 | 0x17 | 0x0F | 0x1f | 0x1b | 0x03 | 0x13 =>         {self.uslo(&opcode.mode)},
                /* SRE */ 0x47 | 0x57 | 0x4F | 0x5f | 0x5b | 0x43 | 0x53 =>         {self.usre(&opcode.mode)},
                /* TAS */ 0x9b =>                                                   {self.utas()},
                /* XAA */ 0x8b =>                                                   {self.uxaa(&opcode.mode)},
                
                /* NOPs */ 0x1a | 0x3a | 0x5a | 0x7a | 0xda | 0xfa =>               {self.unop()},
                /* NOPs */ 0x02 | 0x12 | 0x22 | 0x32 | 0x42 | 0x52 | 0x62 | 0x72 
                | 0x92 | 0xb2 | 0xd2 | 0xf2 =>                                      {self.unop()},
                /* NOP read */ 0x04 | 0x44 | 0x64 | 0x14 | 0x34 | 0x54 | 0x74 
                | 0xd4 | 0xf4 | 0x0c | 0x1c| 0x3c | 0x5c | 0x7c | 0xdc | 0xfc =>    {self.unop_read(&opcode.mode)},
            }

            if program_counter_state == self.register_pc {
                self.register_pc += (opcode.len - 1) as u16;
            }
        }
    }

    // Load a specific program and run from there
    pub fn load_and_run(&mut self, program: Vec<u8>) {
        self.load(program);
        self.reset();
        self.run();
    }

    /*                       */
    /* Opcode Helper Methods */
    /*                       */

    // Add Register A a value and set flags
    // Helper Method for ADC and SBC
    fn add_to_reg_a(&mut self, data: u8) {
        let sum = self.register_a as u16
            + data as u16 
            + self.flags.carry() as u16;
        
        let carry = sum > 0xFF;
        self.flags.set_carry(carry);

        let result = sum as u8;
        self.flags.set_overflow((data ^ result) & (result ^ self.register_a) & 0x80 != 0);

        self.register_a = result;
        self.update_flags(self.register_a);
    }

    // Subtract a value from the A Register
    fn sub_from_reg_a(&mut self, data: u8) {
        self.add_to_reg_a(((data as i8).wrapping_neg().wrapping_sub(1)) as u8);
        self.update_flags(self.register_a);
    }

    // AND a value with the A Register
    fn and_with_reg_a(&mut self, data: u8) {
        self.register_a = data & self.register_a;
        self.update_flags(self.register_a);
    }

    // OR a value with the A Register
    fn or_with_reg_a(&mut self, data: u8) {
        self.register_a = data | self.register_a;
        self.update_flags(self.register_a);
    }

    // XOR a value with the A Register
    fn xor_with_reg_a(&mut self, data: u8) {
        self.register_a = data ^ self.register_a;
        self.update_flags(self.register_a);
    }

    // Branch function to change program counter based on conditions
    fn branch(&mut self, condition: bool) {
        if condition {
            let displacement: i8 = self.mem_read(self.register_pc) as i8;
            let addr = self.register_pc.wrapping_add(1).wrapping_add(displacement as u16);

            self.register_pc = addr;
        }
    }

    // Compare register with a byte of memory
    fn compare(&mut self, mode: &AddressingMode, compare_with: u8) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);

        self.flags.set_carry(compare_with >= data);
        self.update_flags(compare_with.wrapping_sub(data));
    }

    // Get the the address of operands
    pub fn get_absolute_address(&self, mode: &AddressingMode, addr: u16) -> u16 {
        match mode {
            AddressingMode::ZeroPage => self.mem_read(addr) as u16,

            AddressingMode::Absolute => self.mem_read_16(addr),

            AddressingMode::ZeroPageX => {
                let pos = self.mem_read(addr);
                let addr = pos.wrapping_add(self.register_x) as u16;
                addr
            }
            AddressingMode::ZeroPageY => {
                let pos = self.mem_read(addr);
                let addr = pos.wrapping_add(self.register_y) as u16;
                addr
            }

            AddressingMode::AbsoluteX => {
                let base = self.mem_read_16(addr);
                let addr = base.wrapping_add(self.register_x as u16);
                addr
            }
            AddressingMode::AbsoluteY => {
                let base = self.mem_read_16(addr);
                let addr = base.wrapping_add(self.register_y as u16);
                addr
            }

            AddressingMode::IndirectX => {
                let base = self.mem_read(addr);

                let ptr: u8 = (base as u8).wrapping_add(self.register_x);
                let lo = self.mem_read(ptr as u16);
                let hi = self.mem_read(ptr.wrapping_add(1) as u16);
                (hi as u16) << 8 | (lo as u16)
            }
            AddressingMode::IndirectY => {
                let base = self.mem_read(addr);

                let lo = self.mem_read(base as u16);
                let hi = self.mem_read((base as u8).wrapping_add(1) as u16);
                let deref_base = (hi as u16) << 8 | (lo as u16);
                let deref = deref_base.wrapping_add(self.register_y as u16);
                deref
            }

            _ => {
                panic!("mode {:?} is not supported", mode);
            }
        }
    }

    fn get_operand_address(&self, mode: &AddressingMode) -> u16 {
        match mode {
            AddressingMode::Immediate => self.register_pc,
            _ => self.get_absolute_address(mode, self.register_pc),
        }
    }

    // Push Value to Stack
    fn stack_push(&mut self, data: u8) {
        self.mem_write((STACK as u16) + self.register_sp as u16, data);
        self.register_sp = self.register_sp.wrapping_sub(1);
    }

    // Pop Value from the Stack
    fn stack_pop(&mut self) -> u8 {
        self.register_sp = self.register_sp.wrapping_add(1);

        return self.mem_read((STACK as u16) + self.register_sp as u16)
    }

    // Push 2 Byte Value to the Stack
    fn stack_push_16(&mut self, data: u16) {
        let hi = (data >> 8) as u8;
        let lo = (data & 0xFF) as u8;
        self.stack_push(hi);
        self.stack_push(lo);
    }

    // Pop 2 Byte Value from the Stack
    fn stack_pop_16(&mut self) -> u16 {
        let lo = self.stack_pop() as u16;
        let hi = self.stack_pop() as u16;

        return hi << 8 | lo
    }

    // Set Zero and Negative Flags from result
    fn update_flags(&mut self, result: u8) {
        // Set Zero
        self.flags.set_zero(result == 0);

        // Set Negative
        self.flags.set_negative(result & 0b1000_0000 != 0);
    }

    /*           */
    /*  Opcodes  */
    /*           */

    // TODO: FIX FLAGS

    // Add value to register A with the carry bit
    fn adc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.add_to_reg_a(value);
    }

    // Logical AND performed bit by bit on the A Register using a byte of memory
    fn and(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);
        self.register_a = self.register_a & value;

        self.update_flags(self.register_a);
    }

    // Shift all bits of the A Register one bit left
    fn asl_a(&mut self) {
        let mut data = self.register_a;
        self.flags.set_carry(data >> 7 == 1);

        data = data << 1;
        self.register_a = data;
        self.update_flags(self.register_a);
    }

    // Shift all bits of the Memory contents one bit left
    fn asl(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        self.flags.set_carry(data >> 7 == 1);

        data = data << 1;
        self.mem_write(addr, data);
        self.update_flags(data);
        return data;
    }

    // Branch if the carry flag is not set
    fn bcc(&mut self) {
        self.branch(!self.flags.carry());
    }

    // Branch if the carry flag is set
    fn bcs(&mut self) {
        self.branch(self.flags.carry());
    }

    // Branch if the result is Equal
    fn beq(&mut self) {
        self.branch(self.flags.zero());
    }

    // Test if one or more bits are set at a memory location
    fn bit(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        let and = self.register_a & data;

        self.flags.set_zero(and == 0);
        self.flags.set_negative(data & 0b1000_0000 > 0);
        self.flags.set_overflow(data & 0b0100_0000 > 0);
    }

    // Branch if the result is negative
    fn bmi(&mut self) {
        self.branch(self.flags.negative());
    }

    // Branch if the result is not equal
    fn bne(&mut self) {
        self.branch(!self.flags.zero());
    }

    // Branch if the result is positve
    fn bpl(&mut self) {
        self.branch(!self.flags.negative());
    }

    // Force the generation of an interrupt request, pushing status to the stack and loading IRQ interrupt vector at $FFFE/F in the PC
    fn brk(&mut self) {
        self.stack_push_16(self.register_pc);
        self.stack_push(self.flags.bits);
        self.register_pc = self.mem_read_16(0xFFFE);
        self.flags.set_bflag(true);
    }

    // Branch if the overflow is not set adding a displacement to the program counter
    fn bvc(&mut self) {
        self.branch(!self.flags.overflow());
    }

    // Branch if the overflow is set adding a displacement to the program counter
    fn bvs(&mut self) {
        self.branch(self.flags.overflow());
    }

    // Set Carry Flag to False
    fn clc(&mut self) {
        self.flags.set_carry(false);
    }

    // Set Decimal Mode to False
    fn cld(&mut self) {
        self.flags.set_decimal(false);
    }

    // Set Interrupt Disable to False
    fn cli(&mut self) {
        self.flags.set_int(false);
    }

    // Clear the Overflow Flag
    fn clv(&mut self) {
        self.flags.set_overflow(false);
    }

    // Compare the A Register with another byte of memory
    fn cmp(&mut self, mode: &AddressingMode) {
        self.compare(mode, self.register_a);
    }

    // Compare the X Register with another byte of memory
    fn cpx(&mut self, mode: &AddressingMode) {
        self.compare(mode, self.register_x);
    }

    // Compare the Y Register with another byte of memory
    fn cpy(&mut self, mode: &AddressingMode) {
        self.compare(mode, self.register_y);
    }

    // Decrement the value of a byte in memory
    fn dec(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr).wrapping_sub(1);
        self.mem_write(addr, data);
        self.update_flags(data)
    }

    // Decrement the X Register
    fn dex(&mut self) {
        self.register_x = self.register_x.wrapping_sub(1);
        self.update_flags(self.register_x)
    }

    // Decrement the Y Register
    fn dey(&mut self) {
        self.register_y = self.register_y.wrapping_sub(1);
        self.update_flags(self.register_y)
    }

    // Exclusive OR performed bit by bit on the A register using a byte of memory
    fn eor(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);

        self.register_a = self.register_a ^ data;
        self.update_flags(self.register_a);
    }

    // Increment the value stored at a specific memory location
    fn inc(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr).wrapping_add(1);
        self.mem_write(addr, data);
        self.update_flags(data);

        return data;
    }

    // Increment X Register
    fn inx(&mut self) {
        self.register_x = self.register_x.wrapping_add(1);
        self.update_flags(self.register_x);
    }

    // Increment Y Register
    fn iny(&mut self) {
        self.register_y = self.register_y.wrapping_add(1);
        self.update_flags(self.register_y)
    }

    // Jump to a specific program counter address
    fn jmp_abs(&mut self) {
        let addr = self.mem_read_16(self.register_pc);
        self.register_pc = addr;
    }

    // Jump to a specific program counter address
    fn jmp_ind(&mut self) {
        let addr = self.mem_read_16(self.register_pc);

        // Fixes a bug on older CPUs
        let indirect_ref = if addr & 0x00FF == 0x00FF {
            let lo = self.mem_read(addr);
            let hi = self.mem_read(addr & 0xFF00);
            (hi as u16) << 8 | (lo as u16)
        } else {self.mem_read_16(addr)};

        self.register_pc = indirect_ref;
    }

    // Jump to the subroutine and store current address on the stack
    fn jsr(&mut self) {
        self.stack_push_16(self.register_pc + 1);
        let addr = self.mem_read_16(self.register_pc);
        self.register_pc = addr;
    }
    
    // Load the A register using a byte of memory
    fn lda(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_a = value;
        self.update_flags(self.register_a);
    }

    // Load the X Register using a byte of memory
    fn ldx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_x = value;

        self.update_flags(self.register_x);
    }

    // Load the Y Register using a byte of memory
    fn ldy(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let value = self.mem_read(addr);

        self.register_y = value;

        self.update_flags(self.register_y);
    }

    // Logical Shift A Register bits right one place
    fn lsr_a(&mut self) {
        let data = self.register_a;
        self.flags.set_carry(data & 1 == 1);

        self.register_a = data >> 1;
        self.update_flags(self.register_a);

    }

    // Logical Shift bits right one place
    fn lsr(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        self.flags.set_carry(data & 1 == 1);

        data = data >> 1;
        self.mem_write(addr, data);
        self.update_flags(data);

        return data;
    }

    // No Operation, do nothing
    fn nop(&self) {
        // Do Nothing
    }

    // Logical OR performed bit by bit on the A Register using a byte of memory
    fn ora(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);

        self.register_a = self.register_a | data;
        self.update_flags(self.register_a);
    }

    // Push A Register to the stack
    fn pha(&mut self) {
        self.stack_push(self.register_a);
    }

    // Push a copy of the status flags onto the stack
    fn php(&mut self) {
        let mut flags = self.flags.clone();
        flags.set_bflag(true);
        flags.set_uflag(true);
        self.stack_push(flags.bits);
    }

    // Pull an 8 bit value from the stack into the A register
    fn pla(&mut self) {
        let data = self.stack_pop();
        self.register_a = data;
        self.update_flags(self.register_a);
    }

    // Pull an 8 bit value from the stack into the processor flags
    fn plp(&mut self) {
        self.flags.bits = self.stack_pop();
        self.flags.set_bflag(false);
        self.flags.set_uflag(true);
    }

    // Rotate A Register bits to the left
    fn rol_a(&mut self) {
        let mut data = self.register_a;
        let old_carry = self.flags.carry() as u8;

        self.flags.set_carry(data >> 7 == 1);
        data = data << 1;
        data = data | old_carry;

        self.register_a = data;
        self.update_flags(self.register_a);
    }

    // Rotate bits to the left
    fn rol(&mut self, mode: &AddressingMode) -> u8{
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        let old_carry = self.flags.carry() as u8;

        self.flags.set_carry(data >> 7 == 1);
        data = data << 1;
        data = data | old_carry;

        self.mem_write(addr, data);
        self.update_flags(data);

        return data;
    }

    // Rotate A Register bits to the Right
    fn ror_a(&mut self) {
        let mut data = self.register_a;
        let old_carry = self.flags.carry();

        self.flags.set_carry(data & 1 == 1);
        data = data >> 1;
        if old_carry {
            data = data | 0b1000_0000;
        }

        self.register_a = data;
        self.update_flags(self.register_a);
    }

    // Rotate bits to the right
    fn ror(&mut self, mode: &AddressingMode) -> u8 {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);
        let old_carry = self.flags.carry();

        self.flags.set_carry(data & 1 == 1);
        data = data >> 1;
        if old_carry {
            data = data | 0b1000_0000;
        }

        self.mem_write(addr, data);
        self.update_flags(data);

        return data;
    }

    // Return from an Interrupt processing routine to the address stored on the stack
    fn rti(&mut self) {
        self.flags.bits = self.stack_pop();
        self.flags.set_bflag(false);
        self.flags.set_uflag(true);

        self.register_pc = self.stack_pop_16()
    }

    // Return from a subroutine to the pointer stored on the stack
    fn rts(&mut self) {
        self.register_pc = self.stack_pop_16() + 1;
    }

    // Add value to register A with the carry bit
    fn sbc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(&mode);
        let data = self.mem_read(addr);
        
        self.add_to_reg_a(((data as i8).wrapping_neg().wrapping_sub(1)) as u8);
    }

    // Set Carry Flag to True
    fn sec(&mut self) {
        self.flags.set_carry(true);
    }

    // Set Decimal Mode to True
    fn sed(&mut self) {
        self.flags.set_decimal(true);
    }

    // Set Interrupt Disable to True
    fn sei(&mut self) {
        self.flags.set_int(true);
    }

    // Copy value from A to memory
    fn sta(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_a);
    }

    // Store X Register at address
    fn stx(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_x);
    }

    // Store Y Register at address
    fn sty(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, self.register_y);
    }

    // Transfer the contents of the A register to the X register
    fn tax(&mut self) {
        self.register_x = self.register_a;
        self.update_flags(self.register_x);
    }

    // Transfer the contents of the A register to the Y register
    fn tay(&mut self) {
        self.register_y = self.register_a;
        self.update_flags(self.register_y);
    }

    // Transfer the contents of the Stack Pointer to the X register
    fn tsx(&mut self) {
        self.register_x = self.register_sp;
        self.update_flags(self.register_x);
    }

    // Transfer the contents of the X register to the A register
    fn txa(&mut self) {
        self.register_a = self.register_x;
        self.update_flags(self.register_a);
    }

    // Transfer the contents of the X register to the Stack Pointer
    fn txs(&mut self) {
        self.register_sp = self.register_x;
    }

    // Transfer the contents of the Y register to the A register
    fn tya(&mut self) {
        self.register_a = self.register_y;
        self.update_flags(self.register_a);
    }

    /*                    */
    /* Unofficial OPCodes */
    /*                    */

    // Store A & X & Hi+1
    fn uahx_ay(&mut self) {
        let addr = self.mem_read_16(self.register_pc) + self.register_y as u16;
        let data = self.register_a & self.register_x & (addr >> 8) as u8;
        self.mem_write(addr, data);
    }
    
    // Store A & X & Hi+1
    fn uahx_iy(&mut self) {
        let pos = self.mem_read(self.register_pc);
        let addr = self.mem_read_16(pos as u16) + self.register_y as u16;
        let data = self.register_a & self.register_x & (addr >> 8) as u8;
        self.mem_write(addr, data);
    }
    
    // Memory byte AND A then Shift Right A Register Bits
    fn ualr(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        self.and_with_reg_a(data);
        self.lsr_a();
    }
    
    // Memory Byte AND A then set carry to negative value
    fn uanc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        self.and_with_reg_a(data);
        self.flags.set_carry(self.flags.negative());
    }
    
    // Memory Byte AND A then ROR with special flag setting
    fn uarr(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        self.and_with_reg_a(data);

        self.ror_a();
        let result = self.register_a;
        let bit5 = (result >> 5) & 1;
        let bit6 = (result >> 6) & 1;
        self.flags.set_carry(bit6 == 1);
        self.flags.set_overflow(bit5 ^ bit6 == 1);
        self.update_flags(result);

    }
    
    // Set X Register to (X & A) - Memory Byte
    fn uaxs(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);

        let x_and_a = self.register_a & self.register_x;
        let result = x_and_a.wrapping_sub(data);

        self.flags.set_carry(data <= x_and_a);
        self.update_flags(result);
        self.register_x = result;
    }
    
    // Decrement Address then Compare with Address
    fn udcp(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);

        data = data.wrapping_sub(1);
        self.mem_write(addr, data);
        self.flags.set_carry(data <= self.register_a);
        self.update_flags(self.register_a.wrapping_sub(data));
    }

    // Increment Address the Subtract from A Register
    fn uisb(&mut self, mode: &AddressingMode) {
        let data = self.inc(mode);
        self.sub_from_reg_a(data);
    }
    
    // Memory Byte & Stack Pointer, save to A, X, SP
    fn ulas(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let mut data = self.mem_read(addr);

        data = data & self.register_sp;
        self.register_a = data;
        self.register_x = data;
        self.register_sp = data;
        self.update_flags(data);
    }
    
    // Load A then Load X
    fn ulax(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        self.register_a = data;
        self.register_x = self.register_a;
        self.update_flags(self.register_a);
    }
    
    // Load A and transfer to X
    fn ulxa(&mut self, mode: &AddressingMode) {
        self.lda(mode);
        self.tax();
    }
    
    // Do Nothing
    fn unop(&mut self) {
        // Do Nothing
    }
    
    // Read Address, Do Nothing
    fn unop_read(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let _data = self.mem_read(addr);
    }
    
    // Rotate Left and AND with A Register
    fn urla(&mut self, mode: &AddressingMode) {
        let data = self.rol(mode);
        self.and_with_reg_a(data);
    }

    // Rotate Right and Add with Carry to A Register
    fn urra(&mut self, mode: &AddressingMode) {
        let data = self.ror(mode);
        self.add_to_reg_a(data);
    }
    
    // Store A AND X into addr
    fn usax(&mut self, mode: &AddressingMode) {
        let data = self.register_a & self.register_x;
        let addr = self.get_operand_address(mode);
        self.mem_write(addr, data);
    }
    
    // Subtract from Reg A
    fn usbc(&mut self, mode: &AddressingMode) {
        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        self.sub_from_reg_a(data);
    }
    
    // X & 2 Byte Address stored in memory
    fn ushx(&mut self) {
        let addr = self.mem_read_16(self.register_pc) + self.register_y as u16;
        // todo if cross page boundary { addr &= (self.x as u16) << 8}
        let data = self.register_x & ((addr >> 8) as u8 + 1);
        self.mem_write(addr, data);
    }
    
    // Y & 2 Byte address stored in memory
    fn ushy(&mut self) {
        let addr = self.mem_read_16(self.register_pc) + self.register_x as u16;
        // todo if cross page boundary { addr &= (self.x as u16) << 8}
        let data = self.register_y & ((addr >> 8) as u8 + 1);
        self.mem_write(addr, data);
    }
    
    // 2 Byte Do Nothing
    fn uskb(&mut self) {
        // Do Nothing
    }
    
    // Shift bits left and then or with A Register
    fn uslo(&mut self, mode: &AddressingMode) {
        let data = self.asl(mode);
        self.or_with_reg_a(data);
    }
    
    // Shift bits right and then XOR with A Register
    fn usre(&mut self, mode: &AddressingMode) {
        let data = self.lsr(mode);
        self.xor_with_reg_a(data);
    }
    
    // Store A & X in SP and memory
    fn utas(&mut self) {
        let data = self.register_a & self.register_x;
        self.register_sp = data;

        let addr = self.mem_read_16(self.register_pc) + self.register_y as u16;
        let data = ((addr >> 8) as u8 + 1) & self.register_sp;
        self.mem_write(addr, data);
    }

    // Set A to X then AND with a byte of memory
    fn uxaa(&mut self, mode: &AddressingMode) {
        self.register_a = self.register_x;
        self.update_flags(self.register_a);

        let addr = self.get_operand_address(mode);
        let data = self.mem_read(addr);
        self.and_with_reg_a(data);
    }
}

impl Flags {
    fn new() -> Self {
        Flags { bits: 0x24 }
    }

    fn set_bit(&mut self, bit: u8, value: bool) {
        if value { self.bits |= 1 << bit; }
        else { self.bits &= !(1 << bit); }
    }

    fn get_bit(&self, bit: u8) -> bool {
        (self.bits & (1 << bit)) != 0
    }

    fn carry(&self) -> bool     { self.get_bit(0) }
    fn zero(&self) -> bool      { self.get_bit(1) }
    fn int(&self) -> bool       { self.get_bit(2) }
    fn decimal(&self) -> bool   { self.get_bit(3) }
    fn bflag(&self) -> bool     { self.get_bit(4) }
    fn uflag(&self) -> bool    { self.get_bit(5) }
    fn overflow(&self) -> bool  { self.get_bit(6) }
    fn negative(&self) -> bool  { self.get_bit(7) }

    fn set_carry(&mut self, value: bool)        { self.set_bit(0, value); }
    fn set_zero(&mut self, value: bool)         { self.set_bit(1, value); }
    fn set_int(&mut self, value: bool)          { self.set_bit(2, value); }
    fn set_decimal(&mut self, value: bool)      { self.set_bit(3, value); }
    fn set_bflag(&mut self, value: bool)        { self.set_bit(4, value); }
    fn set_uflag(&mut self, value: bool)       { self.set_bit(5, value); }
    fn set_overflow(&mut self, value: bool)     { self.set_bit(6, value); }
    fn set_negative(&mut self, value: bool)     { self.set_bit(7, value); }
}