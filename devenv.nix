{
  pkgs,
  lib,
  config,
  inputs,
  ...
}:

{
  env.GREET = "mparse";
  packages = [
    pkgs.git
    pkgs.lld
    pkgs.wild
    pkgs.rust-analyzer
    pkgs.nodejs
  ];

  languages.rust = {
    enable = true;
    channel = "nightly";
  };

  enterShell = ''
    git --version
  '';
  
}
