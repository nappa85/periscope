fn main() -> std::io::Result<()> {
    tonic_build::configure()
        .build_client(false)
        .extern_path(".common", "::common::grpc")
        .compile_protos(&["../proto/inner.proto"], &["../proto/"])
}
