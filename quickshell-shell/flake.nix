{
  description = "Quickshell dynamic shell surface comparison";
  inputs.nixpkgs.url = "github:NixOS/nixpkgs/567a49d1913ce81ac6e9582e3553dd90a95587";
  outputs = { self, nixpkgs }: let
    systems = [ "x86_64-linux" "aarch64-linux" ];
    each = f: nixpkgs.lib.genAttrs systems (s: f (import nixpkgs { system = s; }));
  in {
    packages = each (pkgs: { default = pkgs.quickshell; });
    devShells = each (pkgs: { default = pkgs.mkShell { packages = [ pkgs.quickshell pkgs.nodejs ]; }; });
  };
}
