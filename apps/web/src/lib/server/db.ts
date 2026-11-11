import 'server-only';
import pg from 'pg';

const globalForPg = globalThis as unknown as {stashPool?: pg.Pool};

function createPool(): pg.Pool {
  const connectionString = process.env.DATABASE_URL;
  if (!connectionString) throw new Error('DATABASE_URL is not set. Copy .env.example to .env.local and fill it in.');
  const needsSsl = /sslmode=require/.test(connectionString) || /supabase\.co|supabase\.com|neon\.tech/.test(connectionString);
  return new pg.Pool({
    connectionString,
    max: Number(process.env.DATABASE_POOL_SIZE ?? 5),
    ssl: needsSsl ? {rejectUnauthorized: false} : undefined,
  });
}

export function pool(): pg.Pool {
  if (!globalForPg.stashPool) globalForPg.stashPool = createPool();
  return globalForPg.stashPool;
}

export async function query<T extends pg.QueryResultRow = pg.QueryResultRow>(text: string, params: unknown[] = []): Promise<T[]> {
  const result = await pool().query<T>(text, params);
  return result.rows;
}

export async function queryOne<T extends pg.QueryResultRow = pg.QueryResultRow>(text: string, params: unknown[] = []): Promise<T | null> {
  const rows = await query<T>(text, params);
  return rows[0] ?? null;
}
