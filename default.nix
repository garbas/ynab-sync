{ pkgs ? import <nixpkgs> {}
}:

pkgs.stdenv.mkDerivation {
  name = "ynab-import";
  src = ./.;
  buildInputs = with pkgs; [
    ag
    entr
    python37
    python37Packages.setuptools
    travis
  ];
  shellHook = ''
    source $HOME/.poetry/env
    poetry shell
  '';
}
