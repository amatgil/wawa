{
  pkgs ? import <nixpkgs>,
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
  ];

  # Environment variables
  env = {
    RUST_BACKTRACE = "1";
    RUST_SRC_PATH = "${pkgs.rustPlatform.rustLibSrc}";
    # For graphics n stuff
    LD_LIBRARY_PATH = "$LD_LIBRARY_PATH:${pkgs.wayland}/lib:${pkgs.libxkbcommon}/lib:${pkgs.libglvnd}/lib";
  };
}
