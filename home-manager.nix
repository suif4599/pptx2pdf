self:
{ config, lib, pkgs, ... }:
with lib;
let
  cfg = config.programs.pptx2pdf;
  flakePkgs = self.packages.${pkgs.system};
in
{
  options.programs.pptx2pdf = {
    enable = mkEnableOption "pptx2pdf";

    useSystemLibreOffice = mkOption {
      type = types.bool;
      default = false;
      description = "Don't install this flake's LibreOffice; call soffice from PATH at runtime. Use this when the system already provides LibreOffice through another channel, to avoid a second copy.";
    };

    staticLibreOffice = mkOption {
      type = types.bool;
      default = false;
      description = "Install the stripped single-binary static LibreOffice build instead of the regular one (much smaller footprint).";
    };
  };

  config = mkIf cfg.enable {
    assertions = [{
      assertion = !(cfg.useSystemLibreOffice && cfg.staticLibreOffice);
      message = "programs.pptx2pdf: useSystemLibreOffice and staticLibreOffice are mutually exclusive";
    }];

    home.packages = [
      (if cfg.useSystemLibreOffice then flakePkgs.pptx2pdf-system
       else if cfg.staticLibreOffice then flakePkgs.pptx2pdf-static
       else flakePkgs.pptx2pdf)
    ];
  };
}
