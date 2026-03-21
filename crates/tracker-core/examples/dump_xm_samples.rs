use xmrs::module::Module;

fn main() {
    let data = std::fs::read("test.xm").unwrap();
    let module = Module::load_xm(&data).unwrap();

    let first_inst = &module.instrument[0];
    if let xmrs::instrument::InstrumentType::Default(def) = &first_inst.instr_type {
        if let Some(Some(samp)) = def.sample.get(0) {
            println!("Sample name: {}", samp.name);
            println!("Sample loop: {:?}", samp.loop_flag);
            println!("Sample bits: {}", samp.bits());

            if let Some(data) = &samp.data {
                match data {
                    xmrs::sample::SampleDataType::Mono8(v) => {
                        println!("Mono8 data (first 20): {:?}", &v[..20]);
                    }
                    xmrs::sample::SampleDataType::Mono16(v) => {
                        println!("Mono16 data (first 20): {:?}", &v[..20]);
                    }
                    _ => println!("Other format"),
                }
            }
        }
    }
}
