//! Build script for eunomia-distributor.
//!
//! Note: Protobuf compilation is currently disabled. When protoc is available,
//! uncomment the `tonic_build` configuration to generate gRPC code from the
//! `proto/control_plane.proto` file.

fn main() {
    // Protobuf compilation is disabled until protoc is available in the build environment.
    // The gRPC types are defined manually in src/grpc/types.rs for now.
    //
    // To enable protobuf compilation:
    // 1. Install protoc: `brew install protobuf` (macOS)
    // 2. Uncomment the following code:
    //
    // tonic_build::configure()
    //     .build_server(true)
    //     .build_client(true)
    //     .out_dir("src/generated")
    //     .compile_protos(
    //         &["../../proto/control_plane.proto"],
    //         &["../../proto"],
    //     )?;
    //
    // println!("cargo:rerun-if-changed=../../proto/control_plane.proto");
}
