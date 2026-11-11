import pg from 'pg';
import {migrate} from '../scripts/migrate';

export default async function setup() {
  const url = process.env.TEST_DATABASE_URL ?? 'postgres://stash:stash@localhost:5432/stash_test';
  const client = new pg.Client({connectionString: url});
  await client.connect();
  await client.query('drop schema public cascade; create schema public;');
  await client.end();
  await migrate(url, () => {});
}
