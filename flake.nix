# SPDX-License-Identifier: GPL-3.0-or-later
{
  description = "Arkestra: A meandering journey through Lisp and Operating Systems.";

  inputs.nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";

  outputs =
    { self, nixpkgs }:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages."${system}";
    in
    {
      devShells."${system}".default = pkgs.mkShell {
        packages = with pkgs; [
          just
          cargo
        ];
      };
    };
}
