fn main() -> std::io::Result<()> {
    tonic_build::configure()
        .build_client(false)
        .build_server(false)
        .compile_protos(&["../proto/common.proto"], &["../proto/"])
}
