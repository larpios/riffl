use opl_emu::chip::OplChipEmu;

fn main() {
    println!("Testing opl-emu");
    let chip = OplChipEmu::new(44100);
    println!("Created chip: {:?}", chip);
}
