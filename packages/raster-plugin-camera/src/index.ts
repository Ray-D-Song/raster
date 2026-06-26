import { registerPlugin, type PermissionStatus } from "@raster/plugin-core";

export interface PhotoResult {
  uri: string;
  width: number;
  height: number;
  format: "jpeg" | "png" | "webp";
}

export interface CameraPlugin {
  checkPermissions(): Promise<PermissionStatus>;
  requestPermissions(): Promise<PermissionStatus>;
  takePhoto(options?: { quality?: number }): Promise<PhotoResult>;
  pickImage(options?: { quality?: number }): Promise<PhotoResult>;
}

export const Camera = registerPlugin<CameraPlugin>("Camera");