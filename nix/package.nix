{
  lib,
  naersk',
  src,
  release ? true,
}:
naersk'.buildPackage {
  pname = "mind";
  version = "0.1.0";
  inherit src release;
  meta = {
    description = "MCP server exposing a Syncthing-shared notes folder.";
    license = with lib.licenses; [
      asl20
      mit
    ];
    mainProgram = "mind";
  };
}
