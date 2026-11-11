import {describe, expect, it} from 'vitest';
import {normalizeServerUrl} from '../utils/settings';

describe('normalizeServerUrl', () => {
  it.each([
    ['stash.example.com', 'https://stash.example.com'],
    ['https://stash.example.com/library?x=1', 'https://stash.example.com'],
    ['localhost:3000', 'http://localhost:3000'],
    ['  http://127.0.0.1:3000/ ', 'http://127.0.0.1:3000'],
    ['', null],
  ])('%s → %s', (input, expected) => {
    expect(normalizeServerUrl(input)).toBe(expected);
  });
});
