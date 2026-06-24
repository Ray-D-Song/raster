import { rasterGlobal } from "../runtime/index.js";
import {
  readRasterFabricMaterializeDiagnostics,
  resetRasterFabricMaterializeDiagnostics,
} from "../renderer/materialize.js";
import { installBridgeDispatch } from "../../bridge/dispatch.js";
import { installRasterEventHandlers } from "./invoke.js";

installBridgeDispatch();
installRasterEventHandlers();
rasterGlobal.__rasterReadMaterializeDiagnostics = readRasterFabricMaterializeDiagnostics;
rasterGlobal.__rasterResetMaterializeDiagnostics = resetRasterFabricMaterializeDiagnostics;

