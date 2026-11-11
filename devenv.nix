{ pkgs, ... }:
{
  languages.rust = {
    enable = true;
    channel = "stable";
    targets = [ "wasm32-unknown-unknown" ];
  };

  packages = with pkgs; [
    trunk
    binaryen # wasm-opt, for release builds
  ];

  scripts = {
    tui.exec = "cargo run --release";
    web.exec = "trunk serve --open";
    web-build.exec = ''trunk build --release --public-url "''${1:-/}"'';
  };

  enterShell = ''
    echo "rp6-cheatsheet: tui | web | web-build [/public-url/]"
  '';
}
