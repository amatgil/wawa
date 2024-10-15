{
  description = "Default Rust flake (casenc)";
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-24.05";
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };
  outputs =
    { self, nixpkgs, rust-overlay }:
    let
      overlays = [ (import rust-overlay) ];
      supportedSystems = [ "x86_64-linux" ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
      pkgs = (import nixpkgs { system = "x86_64-linux"; inherit overlays; });
    in
    {
      packages = forAllSystems (system: {
        default = pkgs.callPackage ./default.nix { inherit pkgs overlays; };
      });
      devShells = forAllSystems (system: {
        default = pkgs.callPackage ./shell.nix { inherit pkgs overlays; };
      });
    };
}
