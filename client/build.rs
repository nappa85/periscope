fn main() -> std::io::Result<()> {
    tonic_build::configure()
        .build_server(false)
        .compile_protos(&["../proto/inner.proto"], &["../proto/"] as &[&str])
}
