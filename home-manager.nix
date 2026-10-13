self:
{ config, lib, pkgs, ... }:
with lib;
let cfg = config.programs.pptx2pdf; in
{
  options.programs.pptx2pdf.enable = mkEnableOption "pptx2pdf";

  config = mkIf cfg.enable {
    home.packages = [ self.packages.${pkgs.system}.pptx2pdf ];
  };
}
