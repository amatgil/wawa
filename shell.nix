{
  pkgs ? import <nixpkgs>,
  lib,
  overlays
}:
pkgs.mkShell {
  # Pinned packages available in the environment
  packages = with pkgs; [
    #rust-bin.stable.latest.default
    (rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
    cargo-bloat
    rust-analyzer
    pkg-config
    sqlx-cli
    git
    openssl.dev
    systemdLibs.dev
    libiconv openssl pkg-config
  ];

  # Environment variables
  env = {
    RUST_BACKTRACE = "1";
    RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
    PKG_CONFIG_PATH = "${pkgs.openssl.dev}/lib/pkgconfig";
    LD_LIBRARY_PATH = lib.makeLibraryPath [ pkgs.openssl ];
  };

}
