{
  description = "Protocol Substrate development environment";
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    # Rust
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
  };

  outputs = { self, nixpkgs, rust-overlay, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
        lib = pkgs.lib;
        toolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      {
        devShells.default = pkgs.mkShell {
          name = "protocol-substrate";
          nativeBuildInputs = [
            pkgs.protobuf
            pkgs.pkg-config
            # Needed for rocksdb-sys
            pkgs.clang
            pkgs.libclang.lib
            pkgs.rustPlatform.bindgenHook
            # Mold Linker for faster builds (only on Linux)
            (lib.optionals pkgs.stdenv.isLinux pkgs.mold)
          ];
          buildInputs = [
            # Used for DVC
            pkgs.python311
            pkgs.python311Packages.pipx
            # We want the unwrapped version, wrapped comes with nixpkgs' toolchain
            pkgs.rust-analyzer-unwrapped
            # Nodejs for test suite
            pkgs.nodePackages.typescript-language-server
            pkgs.nodejs_18
            pkgs.nodePackages.yarn
            # Finally the toolchain
            toolchain
          ];
          packages = [
            pkgs.cargo-nextest
          ];
          # Environment variables
          RUST_SRC_PATH = "${toolchain}/lib/rustlib/src/rust/library";
          # Runs DVC pull in the fixtures
          # we do not install dvc globally, since it
          # is broken on nixos
          shellHook = ''
            ROOT=$(git rev-parse --show-toplevel)
            cd $ROOT/solidity-fixtures && pipx run dvc pull
            cd $ROOT
            cd $ROOT/substrate-fixtures && pipx run dvc pull
            cd $ROOT
          '';
        };
      });
}
