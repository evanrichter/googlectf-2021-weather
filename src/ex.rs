// all state that the vm keeps
#[derive(Default, Debug, Clone)]
struct State {
    // registers
    r0: i32,
    r1: i32,
    r2: i32,
    r3: i32,
    r4: i32,
    // memory
    mem: Vec<u8>,
}

impl State {
    // memory accesses were always 4 bytes at a time, alignment didn't matter
    fn store(&mut self, dest: i32, src: i32) {
        // log the mem write
        println!("storing --> {:x} to index {:x} {}", src, dest, log_index(dest));

        // get index as usize
        let i = dest as u32 as usize;
        // copy over the little endian bytes
        self.mem[i..i + 4].copy_from_slice(&src.to_le_bytes());
    }

    // read 4 bytes from memory
    fn read(&mut self, src: i32) -> i32 {
        // log the mem read
        println!("reading <-- index {:x} {}", src, log_index(src));

        // index as usize
        let i = src as u32 as usize;
        // copy memory bytes into temp buf
        let mut buf = [0; 4];
        buf.copy_from_slice(&self.mem[i..i + 4]);
        // return value as little endian
        i32::from_le_bytes(buf)
    }

    // debugging
    #[allow(dead_code)]
    fn print_regs(&self) -> String {
        format!(
            "{:04x} {:04x} {:04x} {:04x} {:04x}",
            self.r0, self.r1, self.r2, self.r3, self.r4
        )
    }
}

// when memory is logged, I wanted to annotate certain known ranges
fn log_index(index: i32) -> &'static str {
    match index {
        0x1000..=0x1100 => "[user input]",    // user input "city name"
        0x1190..=0x1290 => "[first pass]",    // input lands here after XOR and add operations
        0x1300..=0x1400 => "[RNG numbers]",   // this range was actually prime numbers but whatever
        0x1800..=0x1900 => "[flag output]",   // points to `flag` global addr in binary, see ghidra
        _ => "",
    }
}

pub fn run() {
    // default inits everything to 0 which is fine, I manually checked for any register reads that
    // could have been uninitialized
    let mut s = State::default();

    // copy program bytes
    s.mem = include_bytes!("../mem").to_vec();
    // extend out to include any reads/writes
    s.mem.extend(&[0; 8000]);

    // the following block was added after I understood the program.
    // it reverses the flag arithmetic and final check
    {
        // make the goodboy buffer
        buffer_create(&mut s);
        let goodboy = s.mem[0x1194..0x1194+0x1c].to_vec();
        println!("goodboy {:x?}", goodboy);

        // make the rng numbers buffer
        generate_buffer(&mut s);
        let numbers = s.mem[0x1388..0x1388+38*2].to_vec();
        println!("numbers {:x?}", numbers);

        // get some collatz numbers
        let mut collatz_nums = Vec::new();
        for c in 0..0x1c {
            s.r0 = c + 1;
            collatz(&mut s);
            collatz_nums.push(s.r0 as u8);
        }
        println!("collatz {:x?}", collatz_nums);

        // generate the winning input
        let mut winning_bytes = Vec::new();
        for ii in 0..0x1c {
            let a = goodboy[ii].wrapping_sub(collatz_nums[ii]) ^ numbers[ii*2];
            winning_bytes.push(a);
        }

        // put the right stuff into user input
        s.mem[0x1000..0x1000+winning_bytes.len()].copy_from_slice(&winning_bytes);
        let input = String::from_utf8(s.mem[0x1000..0x101c].to_vec()).unwrap();
        println!("Winning input: {}", input);
    }

    // run the original virtual machine code
    stage2_main(&mut s);

    // extract the flag out of the machine memory
    let s = String::from_utf8(s.mem[0x1800..0x1820].to_vec()).unwrap();
    println!("Flag: {}", s);
}

// mostly original stage2, with added prints
fn stage2_main(s: &mut State) {
    generate_buffer(s);
    println!("done generating buffer");

    s.r0 = 0x0;
    read_input_byte(s);
    println!("done reading input into first pass");

    buffer_check(s);
    println!("done with 4ee");

    // r0 is 0 if buffer check is correct
    if s.r0 == 0 {
        // print flag
        stage2_28d(s);
        println!("done with 28d");
    } else {
        // I also added this else arm, for debugging. curiously, this was always the branch taken
        // even when I got the input right
        println!("cheating");
        stage2_28d(s);
        println!("done cheating with 28d");
    }
}

fn stage2_105(s: &mut State) {
    loop {
        s.r3 = s.r0;
        s.r3 %= s.r2;
        if s.r3 == 0 {
            s.r1 = 0x0;
        }
        s.r2 = s.r2.wrapping_add(0x1);
        s.r3 = s.r2;
        s.r3 = s.r3.wrapping_mul(s.r3);
        s.r3 = s.r3.wrapping_sub(s.r0);
        s.r3 = s.r3.wrapping_sub(0x1);
        if s.r3 >= 0 {
            break;
        }
    }
}

// generates the same buffer every time, that's all I needed to know to solve
// I guess they are prime numbers
fn generate_buffer(s: &mut State) {
    // buf to write to
    s.r4 = 0x1388;

    // counter
    s.r0 = 0x3390;
    while s.r0 < 0x3520 {
        s.r1 = 0x1;
        s.r2 = 0x2;
        stage2_105(s);
        if s.r1 > 0 {
            s.store(s.r4, s.r0);
            s.r4 = s.r4.wrapping_add(0x2);
        }
        s.r0 += 1;
    }
}

// r0 is input index + 1
fn collatz_helper(s: &mut State) {
    s.r1 = s.r0;
    s.r1 %= 0x2;
    if s.r1 == 0 {
        // even index
        s.r0 /= 0x2;
    }
    if s.r1 > 0 {
        // odd index
        s.r0 = s.r0 * 3 + 1;
    }
    collatz(s);
    s.r0 += 1;
}

// r0 is input index + 1
fn collatz(s: &mut State) {
    s.r1 = s.r0 - 1;
    if s.r1 == 0 {
        s.r0 = 0x0;
    } else {
        collatz_helper(s);
    }
}

// r0 is index, starts at 0
// r4 is byte value read
fn read_input_byte(s: &mut State) {
    s.r2 = 0x1000 + s.r0;
    s.r4 = s.read(s.r2);
    s.r4 &= 0xff;

    // input nul byte check
    if s.r4 > 0 {
        process_input_byte(s);
    }
}

// r0 is input index (starts at 0)
// r4 is input byte
fn process_input_byte(s: &mut State) {
    // index r2 into the static buffer and read a byte
    s.r2 = s.read(s.r0 * 2 + 0x1338) & 0xff;

    // xor with input byte
    s.r4 ^= s.r2;

    // increment index, save in r2
    s.r0 += 1;
    s.r2 = s.r0;

    // calculate collatz conjecture and mix into r4
    collatz(s);
    s.r4 = s.r4.wrapping_add(s.r0);
    s.r4 &= 0xff;

    // restore index to r0
    s.r0 = s.r2;
    s.r2 = s.r2.wrapping_sub(0x1);
    s.r2 = s.r2.wrapping_add(0x1194);

    // store to 1194 buf (first pass done?)
    s.store(s.r2, s.r4);
    read_input_byte(s);
}

// final flag output stage. I think it xors the "first pass" buffer then writes to the flag array.
fn stage2_28d(s: &mut State) {
    s.r0 = 0x75bcd15;
    s.r1 = s.read(0x1000);
    s.r0 ^= s.r1;
    s.r2 = 0x3278f102;
    s.r2 ^= s.r0;

    s.r1 = 0x1800;
    s.store(0x1800, s.r2);

    s.r1 = 0x1004;
    s.r1 = s.read(s.r1);
    s.r0 ^= s.r1;

    s.r2 = 0x560aa747;
    s.r2 ^= s.r0;
    s.store(0x1804, s.r2);

    s.r1 = 0x8;
    s.r1 = s.r1.wrapping_add(0x1000);
    s.r1 = s.read(s.r1);
    s.r0 ^= s.r1;
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0x3e6fd176);
    s.r2 ^= s.r0;
    s.r1 = 0x8;
    s.r1 = s.r1.wrapping_add(0x1800);
    s.store(s.r1, s.r2);
    s.r1 = 0xc;
    s.r1 = s.r1.wrapping_add(0x1000);
    s.r1 = s.read(s.r1);
    s.r0 ^= s.r1;
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0x156d86fa);
    s.r2 = s.r2.wrapping_add(0x66c93320);
    s.r2 ^= s.r0;
    s.r1 = 0xc;
    s.r1 = s.r1.wrapping_add(0x1800);
    s.store(s.r1, s.r2);
    s.r1 = 0x10;
    s.r1 = s.r1.wrapping_add(0x1000);
    s.r1 = s.read(s.r1);
    s.r0 ^= s.r1;
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0xe5dbc23);
    s.r2 ^= s.r0;
    s.r1 = 0x10;
    s.r1 = s.r1.wrapping_add(0x1800);
    s.store(s.r1, s.r2);
    s.r1 = 0x14;
    s.r1 = s.r1.wrapping_add(0x1000);
    s.r1 = s.read(s.r1);
    s.r0 ^= s.r1;
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0xd3f894c);
    s.r2 ^= s.r0;
    s.r1 = 0x14;
    s.r1 = s.r1.wrapping_add(0x1800);
    s.store(s.r1, s.r2);
    s.r1 = 0x18;
    s.r1 = s.r1.wrapping_add(0x1000);
    s.r1 = s.read(s.r1);
    s.r0 ^= s.r1;
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0x324fe212);
    s.r2 ^= s.r0;
    s.r1 = 0x18;
    s.r1 = s.r1.wrapping_add(0x1800);
    s.store(s.r1, s.r2);
}

// takes no input
// only uses r0-r2
fn buffer_check(s: &mut State) {
    s.r0 = 0x0;

    // read 4 bytes of first pass buffer
    s.r1 = 0x1194;
    s.r1 = s.read(s.r1);
    s.r2 = 0x51eddb21;
    s.r2 = s.r2.wrapping_add(0x648c4a88);
    s.r2 = s.r2.wrapping_add(0x4355a74c);
    s.r1 ^= s.r2;
    s.r0 |= s.r1;
    s.r1 = 0x4;
    s.r1 = s.r1.wrapping_add(0x1194);
    s.r1 = s.read(s.r1);
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0x32333645);
    s.r2 = s.r2.wrapping_add(0x58728e64);
    s.r1 ^= s.r2;
    s.r0 |= s.r1;
    s.r1 = 0x8;
    s.r1 = s.r1.wrapping_add(0x1194);
    s.r1 = s.read(s.r1);
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0x6f57a0a3);
    s.r1 ^= s.r2;
    s.r0 |= s.r1;
    s.r1 = 0xc;
    s.r1 = s.r1.wrapping_add(0x1194);
    s.r1 = s.read(s.r1);
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0x22d9bbcc);
    s.r2 = s.r2.wrapping_add(0x569fcabc);
    s.r1 ^= s.r2;
    s.r0 |= s.r1;
    s.r1 = 0x10;
    s.r1 = s.r1.wrapping_add(0x1194);
    s.r1 = s.read(s.r1);
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0xd531548);
    s.r1 ^= s.r2;
    s.r0 |= s.r1;
    s.r1 = 0x14;
    s.r1 = s.r1.wrapping_add(0x1194);
    s.r1 = s.read(s.r1);
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0x74c2318e);
    s.r2 = s.r2.wrapping_add(0x7233f6a3);
    s.r1 ^= s.r2;
    s.r0 |= s.r1;
    s.r1 = 0x18;
    s.r1 = s.r1.wrapping_add(0x1194);
    s.r1 = s.read(s.r1);
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0x6d12a1c5);
    s.r2 = s.r2.wrapping_add(0x6c3422b6);
    s.r2 = s.r2.wrapping_add(0xf213d9a);
    s.r1 ^= s.r2;
    s.r0 |= s.r1;

    // r0 should be 0 if the whole buffer was correct
}

// creates the good boy buffer at 0x1194
// I copy + pasted the above function and changed the xor operations with a mem write so I could
// simply extract the correct values :)
// This function is not called by the original program
fn buffer_create(s: &mut State) {
    s.r1 = 0x1194;
    s.r2 = 0x51eddb21;
    s.r2 = s.r2.wrapping_add(0x648c4a88);
    s.r2 = s.r2.wrapping_add(0x4355a74c);
    s.store(s.r1, s.r2);
    s.r1 = 0x4;
    s.r1 = s.r1.wrapping_add(0x1194);
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0x32333645);
    s.r2 = s.r2.wrapping_add(0x58728e64);
    s.store(s.r1, s.r2);
    s.r1 = 0x8;
    s.r1 = s.r1.wrapping_add(0x1194);
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0x6f57a0a3);
    s.store(s.r1, s.r2);
    s.r1 = 0xc;
    s.r1 = s.r1.wrapping_add(0x1194);
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0x22d9bbcc);
    s.r2 = s.r2.wrapping_add(0x569fcabc);
    s.store(s.r1, s.r2);
    s.r1 = 0x10;
    s.r1 = s.r1.wrapping_add(0x1194);
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0xd531548);
    s.store(s.r1, s.r2);
    s.r1 = 0x14;
    s.r1 = s.r1.wrapping_add(0x1194);
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0x74c2318e);
    s.r2 = s.r2.wrapping_add(0x7233f6a3);
    s.store(s.r1, s.r2);
    s.r1 = 0x18;
    s.r1 = s.r1.wrapping_add(0x1194);
    s.r2 = 0x0;
    s.r2 = s.r2.wrapping_add(0x6d12a1c5);
    s.r2 = s.r2.wrapping_add(0x6c3422b6);
    s.r2 = s.r2.wrapping_add(0xf213d9a);
    s.store(s.r1, s.r2);
}
