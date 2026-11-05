fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (proto_file, include_dir) = if std::path::Path::new("proto/inventory.proto").exists() {
        ("proto/inventory.proto", "proto")
    } else {
        ("../proto/inventory/inventory.proto", "../proto/inventory")
    };

    println!("cargo:rerun-if-changed={}", proto_file);
    tonic_build::configure()
        .type_attribute(".", "#[derive(serde::Serialize, serde::Deserialize)]")
        .build_server(true)
        .build_client(true)
        .compile_protos(&[proto_file], &[include_dir])?;

    Ok(())
}
