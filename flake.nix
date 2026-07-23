{
  description = "ofc-engine: OFC ポーカーのコアロジックライブラリの開発環境";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      rust-overlay,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ rust-overlay.overlays.default ];
        };

        # rust-toolchain.toml を単一の真実として toolchain を構築する
        # (chinese-poker-solver と同じ方式。理由はそちらの flake コメント参照)。
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
      in
      {
        devShells.default = pkgs.mkShell {
          packages = [
            rustToolchain
            pkgs.cargo-nextest
            # cargo-watch は現行 nixpkgs pin で darwin ビルドが壊れているため外している
            # (mac-notification-sys のリンクエラー)。必要になったら pin 更新時に再試行。

            # wasm 配布層のビルド(ADR 0002)。wasm-bindgen-cli は Cargo.lock の
            # wasm-bindgen と版が一致している必要がある(不一致はビルドエラーになる)。
            pkgs.wasm-pack
            pkgs.wasm-bindgen-cli
            pkgs.binaryen # wasm-opt

            # packages/engine(薄い TS クライアント)の型チェック用
            pkgs.nodejs_22
            pkgs.typescript
          ];

          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";
        };
      }
    );
}
