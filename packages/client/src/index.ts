export type { paths } from './kernel.d.ts';
export { UNTYPED_DOORS } from './untyped-doors.ts';
import type { paths } from './kernel.d.ts';
import type { UNTYPED_DOORS } from './untyped-doors.ts';

/** A door of the contract, or one the kernel does not serve yet (UNTYPED_DOORS; the mock kernel answers it today). */
export type Door = keyof paths | (typeof UNTYPED_DOORS)[number];
/** A non-2xx answer: the status and the parsed body (the front reads the 402 price from it). */
export class KernelError extends Error {
  readonly status: number; readonly body: unknown;
  constructor(status: number, body: unknown, text: string) { super(`${status} ${text}`); this.name = 'KernelError'; this.status = status; this.body = body; }
}

export interface KernelClientOptions { baseUrl: string; token?: string; fetch?: typeof fetch }

/** A thin caller of the kernel's public doors. It holds no state and decides nothing (AGENTS.md section 1). */
export class KernelClient {
  private readonly baseUrl: string; private readonly token?: string; private readonly fetcher: typeof fetch;
  constructor(o: KernelClientOptions) { this.baseUrl = o.baseUrl; this.token = o.token; this.fetcher = o.fetch ?? ((...a) => globalThis.fetch(...a)); }
  // The path is checked against the contract's keys; typing the response per door is L4's work, so the result is `any` for now.
  async get(path: Door, query?: Record<string, string>, params?: Record<string, string>): Promise<any> { return this.call('GET', fill(path, params), undefined, query); }
  async post(path: Door, body?: unknown, params?: Record<string, string>): Promise<any> { return this.call('POST', fill(path, params), body); }
  private async call(method: string, path: string, body?: unknown, query?: Record<string, string>): Promise<any> {
    const url = new URL(path, this.baseUrl);
    for (const [k, v] of Object.entries(query ?? {})) url.searchParams.set(k, v);
    const headers: Record<string, string> = { accept: 'application/json', 'content-type': 'application/json' };
    if (this.token) headers.authorization = `Bearer ${this.token}`;
    const res = await this.fetcher(url, { method, headers, body: body === undefined ? undefined : JSON.stringify(body) });
    if (!res.ok) { const text = await res.text(); let b: unknown = text; try { b = JSON.parse(text); } catch { /* keep the text */ } throw new KernelError(res.status, b, text); }
    return res.json();
  }
}

function fill(path: string, params?: Record<string, string>): string {
  return path.replace(/\{([A-Za-z0-9_]+)\}/g, (_, k: string) => {
    const v = params?.[k];
    if (v === undefined) throw new Error(`missing param ${k} for ${path}`);
    return encodeURIComponent(v);
  });
}
