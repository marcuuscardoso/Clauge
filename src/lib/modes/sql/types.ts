import type { SqlDialectKey } from './dialects';

// Re-export `SqlDialectKey` under its legacy name `SqlDriver` so existing
// import sites keep compiling. The canonical definition lives in `./dialects`.
export type { SqlDialectKey as SqlDriver } from './dialects';

export interface SqlConnectionConfig {
  name: string;
  driver: SqlDialectKey;
  host: string;
  port: number;
  database: string;
  username: string;
  password: string;
  ssl: boolean;
}

export interface SqlConnection {
  id: string;
  name: string;
  driver: SqlDialectKey;
  host: string;
  port: number;
  databaseName: string;
  username: string;
  password: string;
  ssl: number;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}

export interface SqlQueryResult {
  columns: string[];
  rows: unknown[][];
  affectedRows: number;
  durationMs: number;
}

export interface SqlResultEntry {
  label: string;
  query: string;
  result: SqlQueryResult | null;
  error: string | null;
  connectionId?: string;  // live pool ID — used when AI auto-connects to a different database
}

export interface TableInfo {
  name: string;
  tableType: string;
  rowCount: number;
}

export interface ColumnInfo {
  name: string;
  dataType: string;
  isNullable: boolean;
  isPrimaryKey: boolean;
  defaultValue: string | null;
}

export interface SqlScript {
  id: string;
  name: string;
  connectionId: string | null;
  databaseName: string;
  query: string;
  sortOrder: number;
  createdAt: string;
  updatedAt: string;
}
