use std::{env, path::PathBuf};
fn main () -> Result<(), Box<dyn std::error::Error>> {
    // tonic_build::compile_protos("./protos/simple_queue.proto")?;

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .out_dir("./src/chapter_1")
        .file_descriptor_set_path(out_dir.join("simple_queue_descriptor.bin"))
        .compile(&["./protos/simple_queue.proto"], &["proto"])?;
    Ok(())
}