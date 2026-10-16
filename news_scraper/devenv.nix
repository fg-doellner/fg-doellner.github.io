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
  languages.javascript = {
    enable = true;
    pnpm = {
      enable = true;
      install.enable = true; # make sure we have html2pug easily accessible
    };
  };
}
