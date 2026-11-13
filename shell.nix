{
  pkgs ? import <nixpkgs> {
    overlays = [
      (import (
        builtins.fetchTarball {
          url = "https://github.com/oxalica/rust-overlay/archive/84edd48f141b0c8910dffdd0296ddd577e829132.tar.gz";
          sha256 = "sha256:0h8j7qz8wlf27cx32g5xbgsihpd3v5psnmz2yhlsjhjyrns4hsx1";
        }
      ))
    ];
  },
}:

pkgs.mkShell rec {
  packages = with pkgs; [
    (rust-bin.stable.latest.default.override {
      extensions = [
        "rust-src"
        "rust-analysis"
        "rust-analyzer"
        "rustfmt"
        "clippy"
      ];
      targets = [ "wasm32-unknown-unknown" ];
    })
    llvmPackages.bintools
    cargo-watch
    cargo-outdated
    pkg-config
    udev
    alsa-lib
    nodejs
    wasm-pack
    openssl
    SDL2
    SDL2_ttf
    SDL2_gfx
  ];

  # Without this graphical frontends can't find the GPU adapters
  LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath packages}";
}
