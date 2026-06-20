export type RasterEventHandler<T = unknown> = (payload: T) => void;
export type RasterQueryHandler<TPayload = unknown, TResult = unknown> = (payload: TPayload) => TResult;

export type RasterSurfaceId = number;
export type RasterNodeTag = number;
export type RasterShadowRevisionId = number;
export type RasterSurfaceGeneration = number;
export type RasterHandlerSlotId = number;
export type RasterNativeChildSetId = number;
export type RasterHandlerSlotKind = "event" | "query";

export type HandlerSlotId = RasterHandlerSlotId;

