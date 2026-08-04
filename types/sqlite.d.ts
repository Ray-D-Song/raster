/**
 * The `node:sqlite` module facilitates working with SQLite databases.
 *
 * This module is only available under the `node:` scheme.
 *
 * @see [source](https://github.com/nodejs/node/blob/v24.3.0/doc/api/sqlite.md)
 */
declare module "node:sqlite" {
  import { Buffer } from "buffer";
  import { URL } from "url";

  export interface DatabaseSyncOptions {
    open?: boolean | undefined;
    readOnly?: boolean | undefined;
    enableForeignKeyConstraints?: boolean | undefined;
    enableDoubleQuotedStringLiterals?: boolean | undefined;
    allowExtension?: boolean | undefined;
    timeout?: number | undefined;
  }

  export interface StatementResultingChanges {
    changes: number | bigint;
    lastInsertRowid: number | bigint;
  }

  export interface StatementRunOptions {
    useBigIntArguments?: boolean | undefined;
  }

  export interface StatementGetOptions {
    useBigIntArguments?: boolean | undefined;
  }

  export interface StatementAllOptions {
    useBigIntArguments?: boolean | undefined;
  }

  export interface StatementIteratorOptions {
    useBigIntArguments?: boolean | undefined;
  }

  export interface StatementColumnMetadata {
    name: string;
    column: string | null;
    table: string | null;
    database: string | null;
    type: string | null;
  }

  export interface AggregateOptions<T = unknown> {
    deterministic?: boolean | undefined;
    directOnly?: boolean | undefined;
    useBigIntArguments?: boolean | undefined;
    varargs?: boolean | undefined;
    start?: T | (() => T) | undefined;
    step: (accumulator: T, ...args: unknown[]) => T;
    result?: (accumulator: T) => unknown;
    inverse?: (accumulator: T, ...args: unknown[]) => T;
  }

  export interface FunctionOptions {
    deterministic?: boolean | undefined;
    directOnly?: boolean | undefined;
    useBigIntArguments?: boolean | undefined;
    varargs?: boolean | undefined;
  }

  export interface BackupOptions {
    source?: string | undefined;
    target?: string | undefined;
    rate?: number | undefined;
    progress?: ((progress: BackupProgress) => void) | undefined;
  }

  export interface BackupProgress {
    totalPages: number;
    remainingPages: number;
  }

  export interface CreateSessionOptions {
    table?: string | undefined;
    db?: string | undefined;
  }

  export interface ApplyChangesetOptions {
    filter?: ((tableName: string) => boolean) | undefined;
    onConflict?: ((conflictType: number) => number) | undefined;
  }

  interface Session {
    changeset(): Uint8Array;
    patchset(): Uint8Array;
    close(): void;
  }

  export class DatabaseSync {
    constructor(
      path: string | Buffer | URL,
      options?: DatabaseSyncOptions
    );
    readonly isOpen: boolean;
    readonly isTransaction: boolean;
    open(): void;
    close(): void;
    [Symbol.dispose](): void;
    exec(sql: string): void;
    prepare(sql: string, options?: StatementRunOptions): StatementSync;
    location(dbName?: string): string | null;
    function(
      name: string,
      options: FunctionOptions,
      fn: (...args: unknown[]) => unknown
    ): void;
    function(name: string, fn: (...args: unknown[]) => unknown): void;
    aggregate<T = unknown>(name: string, options: AggregateOptions<T>): void;
    createSession(options?: CreateSessionOptions): Session;
    applyChangeset(
      changeset: Uint8Array,
      options?: ApplyChangesetOptions
    ): boolean;
    loadExtension(path: string, entryPoint?: string): void;
    enableLoadExtension(allow: boolean): void;
  }

  export class StatementSync {
    readonly expandedSQL: string;
    readonly sourceSQL: string;
    run(...params: unknown[]): StatementResultingChanges;
    get(...params: unknown[]): unknown;
    all(...params: unknown[]): unknown[];
    iterate(...params: unknown[]): IterableIterator<unknown>;
    columns(): StatementColumnMetadata[];
    setAllowBareNamedParameters(allow: boolean): void;
    setAllowUnknownNamedParameters(allow: boolean): void;
    setReadBigInts(readBigInts: boolean): void;
    setReturnArrays(returnArrays: boolean): void;
  }

  export function backup(
    sourceDb: DatabaseSync,
    path: string | Buffer | URL,
    options?: BackupOptions
  ): Promise<number>;

  export namespace constants {
    export const SQLITE_CHANGESET_OMIT: number;
    export const SQLITE_CHANGESET_REPLACE: number;
    export const SQLITE_CHANGESET_ABORT: number;
    export const SQLITE_CHANGESET_DATA: number;
    export const SQLITE_CHANGESET_NOTFOUND: number;
    export const SQLITE_CHANGESET_CONFLICT: number;
    export const SQLITE_CHANGESET_CONSTRAINT: number;
    export const SQLITE_CHANGESET_FOREIGN_KEY: number;
  }
}
