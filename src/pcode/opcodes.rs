// Clean-room re-implementation. Behavior derived from publicly documented
// VB6 / COM / Win32 ABI semantics. No Microsoft or third-party source used.

//! # P-Code Bytecode Opcode Definitions
//!
//! Enumerates and classifies the instruction set architecture (ISA) of the Visual Basic 6.0
//! P-Code Virtual Machine (VM).

/// P-Code Opcode Constants.
pub const OP_NOP: u8 = 0x00;

// Literals and Immediate Push
pub const OP_LITI2: u8 = 0x01;
pub const OP_LITI4: u8 = 0x02;
pub const OP_LITR4: u8 = 0x03;
pub const OP_LITR8: u8 = 0x04;
pub const OP_LITCY: u8 = 0x05;
pub const OP_LITSTR: u8 = 0x06;

// Locals and Parameter Stack Load/Store
pub const OP_LDLOCAL: u8 = 0x10;
pub const OP_STLOCAL: u8 = 0x11;
pub const OP_LDARG: u8 = 0x12;
pub const OP_STARG: u8 = 0x13;
pub const OP_LDARG_BYREF: u8 = 0x14;
pub const OP_STARG_BYREF: u8 = 0x15;
pub const OP_DUP: u8 = 0x18;
pub const OP_POP: u8 = 0x19;

// Arithmetic Operators
pub const OP_ADD: u8 = 0x20;
pub const OP_SUB: u8 = 0x21;
pub const OP_MUL: u8 = 0x22;
pub const OP_DIV: u8 = 0x23;
pub const OP_IDIV: u8 = 0x24;
pub const OP_MOD: u8 = 0x25;
pub const OP_POW: u8 = 0x26;
pub const OP_NEG: u8 = 0x27;

// Relational Comparisons
pub const OP_EQ: u8 = 0x30;
pub const OP_NE: u8 = 0x31;
pub const OP_LT: u8 = 0x32;
pub const OP_LE: u8 = 0x33;
pub const OP_GT: u8 = 0x34;
pub const OP_GE: u8 = 0x35;

// Logical and Bitwise Operations
pub const OP_AND: u8 = 0x40;
pub const OP_OR: u8 = 0x41;
pub const OP_NOT: u8 = 0x42;
pub const OP_XOR: u8 = 0x43;

// Control Flow Branches
pub const OP_BRANCH: u8 = 0x50;
pub const OP_BRANCH_IF_ZERO: u8 = 0x51;
pub const OP_BRANCH_IF_NOT_ZERO: u8 = 0x52;
pub const OP_SELECT_CASE_JUMP: u8 = 0x55;

// Runtime Import and Method Invocations
pub const OP_IMPAD_CALL: u8 = 0x60;
pub const OP_METH_CALL: u8 = 0x61;
pub const OP_PROCCALL: u8 = 0x62;

// SEH Error Trapping
pub const OP_ON_ERROR_GOTO: u8 = 0x70;
pub const OP_ON_ERROR_RESUME_NEXT: u8 = 0x71;
pub const OP_RESUME: u8 = 0x72;

// Procedure Termination
pub const OP_RETURN: u8 = 0xFE;
pub const OP_HALT: u8 = 0xFF;
