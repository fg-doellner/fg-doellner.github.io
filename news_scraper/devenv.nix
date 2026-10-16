{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

{
  packages = with pkgs; [
    jq
  ];

  languages.rust.enable = true;
}
