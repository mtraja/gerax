fn main() {
    tonic_prost_build::configure()
        .build_server(true)
        .build_client(true)
        .compile_protos(&["proto/gerax_rpc.proto"], &["proto"])
        .expect("Failed to compile gerax_rpc.proto");
}
