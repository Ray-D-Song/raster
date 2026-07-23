declare module "diagnostics_channel" {
  export class Channel {
    readonly name: string | symbol;
    readonly hasSubscribers: boolean;
    subscribe(subscription: (message: unknown, name: string | symbol) => void): void;
    unsubscribe(subscription: (message: unknown, name: string | symbol) => void): boolean;
    publish(message: unknown): void;
  }
  export function channel(name: string | symbol): Channel;
  export function subscribe(
    name: string | symbol,
    subscription: (message: unknown, name: string | symbol) => void
  ): void;
  export function unsubscribe(
    name: string | symbol,
    subscription: (message: unknown, name: string | symbol) => void
  ): boolean;
  export function hasSubscribers(name: string | symbol): boolean;

  interface DiagnosticsChannelModule {
    Channel: typeof Channel;
    channel: typeof channel;
    subscribe: typeof subscribe;
    unsubscribe: typeof unsubscribe;
    hasSubscribers: typeof hasSubscribers;
  }
  const diagnostics_channel: DiagnosticsChannelModule;
  export default diagnostics_channel;
}
declare module "node:diagnostics_channel" {
  export * from "diagnostics_channel";
  export { default } from "diagnostics_channel";
}
