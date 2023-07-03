use std::fmt::Display;

pub trait OpCode {
    const OP_CODE: u8;
    const FUNCT: Option<u8>;
}

impl<R, D> OpCode for J<R, D> {
    const OP_CODE: u8 = 0b000010;
    const FUNCT: Option<u8> = None;
}

impl<R, D> OpCode for JAL<R, D> {
    const OP_CODE: u8 = 0b000011;
    const FUNCT: Option<u8> = None;
}

impl<R, D> OpCode for BGEZ<R, D> {
    const OP_CODE: u8 = 0b000001;
    const FUNCT: Option<u8> = None;
}

impl<R, D> OpCode for BGEZAL<R, D> {
    const OP_CODE: u8 = 0b000001;
    const FUNCT: Option<u8> = None;
}

impl<R, D> OpCode for BLTZ<R, D> {
    const OP_CODE: u8 = 0b000001;
    const FUNCT: Option<u8> = None;
}

impl<R, D> OpCode for BLTZAL<R, D> {
    const OP_CODE: u8 = 0b000001;
    const FUNCT: Option<u8> = None;
}

impl<R, D> OpCode for BEQ<R, D> {
    const OP_CODE: u8 = 0b000100;
    const FUNCT: Option<u8> = None;
}

impl<R, D> OpCode for BNE<R, D> {
    const OP_CODE: u8 = 0b000101;
    const FUNCT: Option<u8> = None;
}

impl<R, D> OpCode for BLZE<R, D> {
    const OP_CODE: u8 = 0b000110;
    const FUNCT: Option<u8> = None;
}

impl<R, D> OpCode for BGTZ<R, D> {
    const OP_CODE: u8 = 0b000111;
    const FUNCT: Option<u8> = None;
}

pub enum SyscallNumber {
    MMAP = 4090,
    BRK = 4045,
    CLONE = 4120,
    EXIT = 4246,
    READ = 4003,
    WRITE = 4004,
    FCNTL = 4055,
}

pub enum Instruction {}
