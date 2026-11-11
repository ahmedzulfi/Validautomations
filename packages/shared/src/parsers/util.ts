export type Json = unknown;

export function isObject(v: unknown): v is Record<string, unknown> {
  return typeof v === 'object' && v !== null && !Array.isArray(v);
}

export function str(v: unknown): string | undefined {
  return typeof v === 'string' && v.length > 0 ? v : undefined;
}

/** Visits every object in a JSON tree, depth first. */
export function walk(root: unknown, visit: (obj: Record<string, unknown>) => void, maxDepth = 60): void {
  const stack: Array<[unknown, number]> = [[root, 0]];
  while (stack.length) {
    const [node, depth] = stack.pop()!;
    if (depth > maxDepth) continue;
    if (Array.isArray(node)) {
      for (let i = node.length - 1; i >= 0; i--) stack.push([node[i], depth + 1]);
    } else if (isObject(node)) {
      visit(node);
      const values = Object.values(node);
      for (let i = values.length - 1; i >= 0; i--) stack.push([values[i], depth + 1]);
    }
  }
}

export function toIso(date: Date | number | string | undefined | null): string | undefined {
  if (date === undefined || date === null || date === '') return undefined;
  const d = date instanceof Date ? date : new Date(date);
  return Number.isNaN(d.getTime()) ? undefined : d.toISOString();
}

export function isHttpUrl(v: string | undefined): v is string {
  if (!v) return false;
  try {
    const u = new URL(v);
    return u.protocol === 'http:' || u.protocol === 'https:';
  } catch {
    return false;
  }
}

export function firstLine(text: string | undefined, max = 140): string | undefined {
  if (!text) return undefined;
  const line = text.split('\n').map((l) => l.trim()).find(Boolean);
  if (!line) return undefined;
  return line.length > max ? line.slice(0, max - 1).trimEnd() + '…' : line;
}
