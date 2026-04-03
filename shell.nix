{
  pkgs ? import <nixpkgs> {
    overlays = [
      (import (
        builtins.fetchTarball {
          url = "https://github.com/oxalica/rust-overlay/archive/8d6387ed6d8e6e6672fd3ed4b61b59d44b124d99.tar.gz";
          sha256 = "sha256:0kdd7jpbyr1r578yfq95j317sk9dyrw0yxgnkiny2b43f4d6q46z";
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
