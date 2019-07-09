let
  pkgs_mozilla = import (builtins.fetchTarball https://github.com/mozilla/nixpkgs-mozilla/archive/master.tar.gz);
in

{ pkgs ? import <nixpkgs> { overlays = [ pkgs_mozilla ]; }
}:

let
  rustChannel = (pkgs.rustChannelOf { date = "2019-01-26"; channel = "nightly"; });
  #rustPlatform = (builtins.getAttr rustChannel pkgs.latest.rustChannels);
in
pkgs.stdenv.mkDerivation {
  name = "ynap-sync";
  buildInputs = with pkgs; [
    (rustChannel.rust.override {
       extensions = [
         "clippy-preview"
         "rls-preview"
         "rustfmt-preview"
         "rust-analysis"
         "rust-std"
         "rust-src"
       ];
     })
    cargo-graph
    cargo-edit
    cargo-release
    openssl
    pkgconfig
    carnix
  ];
  shellHook = ''
    export RUST_SRC_PATH="$(rustc --print sysroot)/lib/rustlib/src/rust/src"
  '';
}
