use std::{env, path::PathBuf};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .out_dir("./src/zeyrho")
        .file_descriptor_set_path(out_dir.join("kv_store_descriptor.bin"))
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(&["./protos/kv_store.proto"], &["proto"])?;

    tonic_build::configure()
        .protoc_arg("--experimental_allow_proto3_optional")
        .out_dir("./src/zeyrho")
        .file_descriptor_set_path(out_dir.join("queue_descriptor.bin"))
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .compile_protos(&["./protos/queue.proto"], &["proto"])?;

    Ok(())
}
