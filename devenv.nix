{ pkgs, ... }:

{
  packages = with pkgs; [
    cargo
    rustup
    gdb
  ];
}
