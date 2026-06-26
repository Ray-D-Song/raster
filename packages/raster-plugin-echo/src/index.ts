import { registerPlugin } from "@raster/plugin-core";

export interface EchoResult {
  echo: string;
}

export interface EchoPlugin {
  echo(args: { msg: string }): Promise<EchoResult>;
}

export const Echo = registerPlugin<EchoPlugin>("Echo");