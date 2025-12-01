fn main() -> Result<(), Box<dyn std::error::Error>> {
    tonic_build::configure()
        .build_server(false) // ws-server 只需要客户端
        .build_client(true)
        .compile_protos(&["proto/health.proto"], &["proto"])?;
    Ok(())
}
