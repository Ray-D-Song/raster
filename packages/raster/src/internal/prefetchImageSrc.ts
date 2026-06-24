import { useEffect } from "react";
import { getRasterBridge } from "../bridge/index.js";

type AvatarSpecEntry = string | { src?: string };

const inflightFetches = new Map<string, Promise<ArrayBuffer>>();

export function isRemoteSrc(src: string | undefined): src is string {
  return src != null && (src.startsWith("http://") || src.startsWith("https://"));
}

function fetchRemoteImage(uri: string): Promise<ArrayBuffer> {
  const existing = inflightFetches.get(uri);
  if (existing != null) {
    return existing;
  }

  const task = (async () => {
    const response = await fetch(uri);
    if (!response.ok) {
      throw new Error(`Failed to fetch image: ${response.status}`);
    }
    return response.arrayBuffer();
  })().finally(() => {
    inflightFetches.delete(uri);
  });

  inflightFetches.set(uri, task);
  return task;
}

async function loadRemoteImage(uri: string, cancelled: () => boolean): Promise<void> {
  const bytes = await fetchRemoteImage(uri);
  if (cancelled()) {
    return;
  }
  getRasterBridge().post("host.assets", "load", { uri, bytes });
}

export function usePrefetchImageSrc(src?: string): void {
  useEffect(() => {
    if (!isRemoteSrc(src)) {
      return;
    }

    let cancelled = false;
    void loadRemoteImage(src, () => cancelled).catch((error) => {
      if (!cancelled) {
        console.error("[raster] failed to prefetch image", { uri: src, error });
      }
    });

    return () => {
      cancelled = true;
    };
  }, [src]);
}

function remoteUrisFromSpecs(specs: Array<AvatarSpecEntry> | undefined): string[] {
  if (specs == null) {
    return [];
  }
  const uris = new Set<string>();
  for (const entry of specs) {
    if (typeof entry === "string") {
      continue;
    }
    if (isRemoteSrc(entry.src)) {
      uris.add(entry.src);
    }
  }
  return [...uris];
}

export function usePrefetchAvatarSpecs(specs: Array<AvatarSpecEntry> | undefined): void {
  const uris = remoteUrisFromSpecs(specs).join("\0");

  useEffect(() => {
    if (uris.length === 0) {
      return;
    }

    const uriList = uris.split("\0");
    let cancelled = false;

    for (const uri of uriList) {
      void loadRemoteImage(uri, () => cancelled).catch((error) => {
        if (!cancelled) {
          console.error("[raster] failed to prefetch avatar image", { uri, error });
        }
      });
    }

    return () => {
      cancelled = true;
    };
  }, [uris]);
}