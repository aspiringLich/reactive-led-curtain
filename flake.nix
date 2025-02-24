{
  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    rust-overlay.url = "github:oxalica/rust-overlay";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      rust-overlay,
      flake-utils,
      ...
    }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs {
          inherit system overlays;
        };
      in
      {
        devShells.default = pkgs.mkShell rec {
          packages = with pkgs; [
            openssl
            pkg-config
            alsa-lib
            wayland
            libxkbcommon
            fontconfig
            libGL
            (rust-bin.selectLatestNightlyWith (toolchain: toolchain.default))
          ];
          LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath packages;
        };
      }
    );
}
