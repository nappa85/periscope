fn main() -> std::io::Result<()> {
    tonic_build::configure()
        .build_client(false)
        .compile_protos(&["../proto/inner.proto"], &["../proto/"] as &[&str])
}
