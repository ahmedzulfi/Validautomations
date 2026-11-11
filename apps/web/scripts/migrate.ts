// Applies every SQL file in db/migrations that has not run yet, in name order, each in a transaction.
// Usage: DATABASE_URL=postgres://... pnpm db:migrate
import {readdirSync, readFileSync} from 'node:fs';
import path from 'node:path';
import {fileURLToPath} from 'node:url';
import pg from 'pg';

const here = path.dirname(fileURLToPath(import.meta.url));
const dir = path.join(here, '..', 'db', 'migrations');

export async function migrate(connectionString: string, log = console.log): Promise<void> {
  const client = new pg.Client({connectionString});
  await client.connect();
  try {
    await client.query('create table if not exists schema_migrations (name text primary key, applied_at timestamptz not null default now())');
    const done = new Set((await client.query<{name: string}>('select name from schema_migrations')).rows.map((r) => r.name));
    for (const file of readdirSync(dir).filter((f) => f.endsWith('.sql')).sort()) {
      if (done.has(file)) continue;
      await client.query('begin');
      try {
        await client.query(readFileSync(path.join(dir, file), 'utf8'));
        await client.query('insert into schema_migrations (name) values ($1)', [file]);
        await client.query('commit');
        log(`applied ${file}`);
      } catch (err) {
        await client.query('rollback');
        throw err;
      }
    }
  } finally {
    await client.end();
  }
}

if (process.argv[1] && fileURLToPath(import.meta.url) === path.resolve(process.argv[1])) {
  const url = process.env.DATABASE_URL;
  if (!url) {
    console.error('Set DATABASE_URL first.');
    process.exit(1);
  }
  migrate(url).then(() => console.log('Database is up to date.'), (err) => {
    console.error(err);
    process.exit(1);
  });
}
