let
  sources = import ./nix/sources.nix;
  mozilla_overlay = import sources.nixpkgs-mozilla;
  overlay = _: pkgs: {
    niv = import sources.niv {};
  } // (import sources."gitignore.nix" {});
in

{ pkgs ? import sources.nixpkgs { overlays = [ mozilla_overlay overlay ]; }
}:

let
  naersk = pkgs.callPackage sources.naersk {};
in naersk.buildPackage {
  src = pkgs.gitignoreSource ./.;
  buildInputs = with pkgs; [
    pkgconfig
    openssl

    rustPackages.clippy
    rustPackages.rls
    rustPackages.rustfmt

    cargo-graph
    # TODO: fails to build
    #cargo-edit
    cargo-release

    ag
    entr
  ];
}


#let
#  #rustChannel = (pkgs.rustChannelOf { date = "2019-01-26"; channel = "nightly"; });
#  rustChannel = (pkgs.rustChannelOf { date = "2019-09-13"; channel = "nightly"; });
#  #rustChannel = (builtins.getAttr "nightly" pkgs.latest.rustChannels);
#in
#pkgs.stdenv.mkDerivation {
#  name = "ynap-sync";
#  buildInputs = with pkgs; [
#    (rustChannel.rust.override {
#       extensions = [
#         "clippy-preview"
#         "rls-preview"
#         "rustfmt-preview"
#         "rust-analysis"
#         "rust-std"
#         "rust-src"
#       ];
#     })
#    cargo-graph
#    #cargo-edit
#    cargo-release
#    openssl
#    pkgconfig
#    carnix
#
#    ag
#    entr
#  ];
#  shellHook = ''
#    export RUST_SRC_PATH="$(rustc --print sysroot)/lib/rustlib/src/rust/src"
#  '';
#}
