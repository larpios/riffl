fn main() {
    println!("Testing xmrs types...");
    // Let's just create a dummy module to see if this compiles and what types exist
    // We just want to trigger a compile error that shows the fields of xmrs::instrument::Default
    let _ = xmrs::instrument::InstrumentType::Default(Default::default());
}
