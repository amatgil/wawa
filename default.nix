{
  pkgs ? import <nixpkgs> { },
  overlays
}:
let
  manifest = (pkgs.lib.importTOML ./Cargo.toml).package;
  packages = with pkgs; [
	  rust-bin.stable.latest.default
  ];
in
pkgs.rustPlatform.buildRustPackage {
  pname = manifest.name;
  version = manifest.version;
  cargoLock.lockFile = ./Cargo.lock;
  src = pkgs.lib.cleanSource ./.;
  meta.description = manifest.description ? null;
}
