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

  languages.javascript = {
    enable = true;
    pnpm.enable = true;
  };
}
