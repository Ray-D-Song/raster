import { rasterGlobal } from "../runtime/index.js";
import {
  readRasterFabricMaterializeDiagnostics,
  resetRasterFabricMaterializeDiagnostics,
} from "../renderer/materialize.js";
import { installRasterEventHandlers } from "./invoke.js";

installRasterEventHandlers();
rasterGlobal.__rasterReadMaterializeDiagnostics = readRasterFabricMaterializeDiagnostics;
rasterGlobal.__rasterResetMaterializeDiagnostics = resetRasterFabricMaterializeDiagnostics;

