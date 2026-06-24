import type {
  RasterHandlerSlotId,
  RasterHandlerSlotKind,
  RasterNodeTag,
  RasterSurfaceId,
} from "../types/index.js";
import { rasterGlobal } from "../runtime/global.js";

export type HandlerId = RasterHandlerSlotId;
export type RasterHandler = (payload: unknown) => unknown;
export type HandlerKind = RasterHandlerSlotKind;
export type HandlerSlotKey = string;

export interface HandlerRecord {
  handler: RasterHandler;
  eventType: string | null;
  surfaceId: RasterSurfaceId;
  nodeTag: RasterNodeTag;
  name: string;
}

export interface RasterHandlerRegistry {
  handlers: Map<HandlerId, HandlerRecord>;
  handlerSlots: Map<HandlerSlotKey, HandlerId>;
}

export const handlerRegistry =
  rasterGlobal.__rasterHandlerRegistry ??
  (rasterGlobal.__rasterHandlerRegistry = {
    handlers: new Map<HandlerId, HandlerRecord>(),
    handlerSlots: new Map<HandlerSlotKey, HandlerId>(),
  });

export const handlers = handlerRegistry.handlers;
export const handlerSlots = handlerRegistry.handlerSlots;
export const textControlEventCounts = new Map<string, number>();

export function resetRasterHandlerRegistry(): void {
  handlers.clear();
  handlerSlots.clear();
  textControlEventCounts.clear();
}

