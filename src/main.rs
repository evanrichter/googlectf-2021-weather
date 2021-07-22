// emulation code in ex.rs
mod ex;

#[derive(Debug, Clone, Copy)]
struct Instruction {
    // width
    dest: u32,
    // precision
    src: u32,
    // operand1 mode
    dest_mode: DestMode,
    // operand2 mode
    src_mode: SrcMode,
    // arithmetic to do
    op: Operation,
}

#[derive(Debug, Clone, Copy)]
enum DestMode {
    NoPlusMinus,
    Plus,
    Minus,
    ZeroPad,
}

#[derive(Debug, Clone, Copy)]
enum SrcMode {
    HH,
    H,
    LL,
    L,
    None,
}

#[derive(Debug, Clone, Copy)]
enum Operation {
    Jmp,
    Mov,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    ShLeft,
    ShRight,
    Xor,
    And,
    Or,
    Ret,
}

// this prints the instruction. started out as syntax like "mov r1, [r0]" but then changed to
// output pseudo rust code that only required small fixups in ex.rs to actually execute
impl std::fmt::Display for Instruction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op = match self.op {
            Operation::Jmp => {
                let op = match self.dest_mode {
                    DestMode::Minus => "< 0",
                    DestMode::Plus => "> 0",
                    DestMode::ZeroPad => "== 0",
                    DestMode::NoPlusMinus => return write!(f, "stage2_{:x}(&mut s);", self.dest),
                };

                return write!(f, "if s.r{} {} {{ stage2_{:x}(&mut s); }}", self.src, op, self.dest);
            }
            /*   // old syntax
            Operation::Mov => "mov",
            Operation::Add => "add",
            Operation::Sub => "sub",
            Operation::Mul => "mul",
            Operation::Div => "div",
            Operation::Mod => "mod",
            Operation::ShLeft => "shl",
            Operation::ShRight => "shr",
            Operation::Xor => "xor",
            Operation::And => "and",
            Operation::Or => " or",
            */
            // new syntax
            Operation::Mov => "=",
            Operation::Add => "+=",
            Operation::Sub => "-=",
            Operation::Mul => "*=",
            Operation::Div => "/=",
            Operation::Mod => "%=",
            Operation::ShLeft => "<<=",
            Operation::ShRight => ">>=",
            Operation::Xor => "^=",
            Operation::And => "&=",
            Operation::Or => "|=",
            Operation::Ret => return write!(f, "ret"),
        };

        // write the destination part
        match self.dest_mode {
            DestMode::Minus => write!(f, "[{:#0x}]", self.dest)?,
            DestMode::Plus => write!(f, "[r{}]", self.dest)?,
            DestMode::NoPlusMinus => write!(f, "s.r{}", self.dest)?,
            _ => panic!(),
        }

        // write the opcode
        write!(f, " {} ", op)?;

        // write the source part
        match self.src_mode {
            SrcMode::HH => write!(f, "[{:#0x}];", self.src),
            SrcMode::H => write!(f, "s.mem[s.r{} as u32 as usize];", self.src),
            SrcMode::L => write!(f, "s.r{};", self.src),
            SrcMode::LL => write!(f, "{:#0x};", self.src),
            _ => panic!(),
        }
    }
}

impl Instruction {

    // this parses a string like "%+4.7hhX" and then returns an Instruction as well as where to
    // keep parsing from next
    fn parse(mem: &[u8]) -> (Self, &[u8]) {
        if mem[0] == 0 {
            return (Self {
                dest: 0,
                src: 0,
                dest_mode: DestMode::Minus,
                src_mode: SrcMode::LL,
                op: Operation::Ret,
            }, &mem[1..]);
        }

        assert!(b'%' == mem[0]);
        let mem = &mem[1..];

        // parse mode from flags
        let (op1_mode, mem) = match mem {
            &[b'-', ..] => (DestMode::Minus, &mem[1..]),
            &[b'+', ..] => (DestMode::Plus, &mem[1..]),
            &[b'0', b'.', ..] => (DestMode::NoPlusMinus, mem),
            &[b'0', ..] => (DestMode::ZeroPad, &mem[1..]),
            _ => (DestMode::NoPlusMinus, mem),
        };

        // parse width (operand1)
        let (operand1, mem) = parse_int(mem);

        let (operand2, op2_mode, mem) = if mem[0] == b'.' {
            let mem = &mem[1..];

            let (operand2, mem) = parse_int(mem);

            let (op2_mode, mem) = match mem {
                &[b'h', b'h', .. ] => (SrcMode::HH, &mem[2..]),
                &[b'h', .. ] => (SrcMode::H, &mem[1..]),
                &[b'l', b'l', .. ] => (SrcMode::LL, &mem[2..]),
                &[b'l', .. ] => (SrcMode::L, &mem[1..]),
                _ => (SrcMode::None, mem),
            };
            (operand2, op2_mode, mem)
        } else {
            (0, SrcMode::None, mem)
        };
        
        let operation = match mem[0] {
            b'C' => Operation::Jmp,
            b'M' => Operation::Mov,
            b'S' => Operation::Add,
            b'O' => Operation::Sub,
            b'X' => Operation::Mul,
            b'V' => Operation::Div,
            b'N' => Operation::Mod,
            b'L' => Operation::ShLeft,
            b'R' => Operation::ShRight,
            b'E' => Operation::Xor,
            b'I' => Operation::And,
            b'U' => Operation::Or,
            _ => panic!(),
        };

        (Self {
            dest: operand1,
            src: operand2,
            dest_mode: op1_mode,
            src_mode: op2_mode,
            op: operation,
        }, &mem[1..])
    }
}

fn parse_int(mut mem: &[u8]) -> (u32, &[u8]) {
    let mut val = 0;
    loop {
        let curr = mem[0];
        if curr.is_ascii_digit() {
            val *= 10;
            val += curr as u32 - b'0' as u32;
            mem = &mem[1..];
        } else {
            break;
        }
    }
    (val, mem)
}

fn main() {
    //disassemble();
    ex::run();
}

#[allow(dead_code)]
fn disassemble() {
    // I dumped bytes with ghidra copy + paste to a python interpreter, then wrote to raw bytes
    let mem = include_bytes!("../mem");

    // first instruction is weird, it prints flag
    // note: the reason it's weird is because it has one "real" instruction (a call) then it has a
    // %s which prints the flag and I don't parse that. it's the end of the program anyway
    let (inst, _) = Instruction::parse(mem);
    println!("   0: {}", inst);

    // this part disassembles the first stub. it un-xors the rest of the instructions
    let mut curr: usize = 6;
    while curr < 0xc8 {
        let s = String::from_utf8(mem[curr..curr+20].to_vec()).unwrap();
        let (inst, next) = Instruction::parse(&mem[curr..]);
        println!("{:#04x}: {:30}   {}", curr, s, inst);
        curr = mem.len() - next.len();
    }

    // manually un-xor the second stage
    let key = b'%' ^ mem[0xc8];
    let mut mem = mem.to_vec();
    for ii in 0xc8..0x6fc {
        mem[ii] ^= key;
    }

    // seek to second stage and disassemble
    let mut curr: usize = 0xc8;
    while mem.len() > curr {
        let next = curr+1+mem[curr + 1..].iter().position(|c| *c == b'%' || *c == 0).unwrap_or_else(|| 30);
        let upper = next.min(mem.len());
        let s = String::from_utf8(mem[curr..upper].to_vec()).unwrap();
        let (inst, next) = Instruction::parse(&mem[curr..]);
        println!("{:#05x}:  {}", curr, inst);
        curr = mem.len() - next.len();
    }
}
