export const API_URL = (
  process.env.NEXT_PUBLIC_API_URL ?? "http://localhost:8080"
).replace(/\/+$/, "");

export type ProbeStats = {
  min_ms: number;
  avg_ms: number;
  p50_ms: number;
  p95_ms: number;
  max_ms: number;
};

export type ProbeResult = {
  host: string;
  port: number;
  ip: string;
  dns_ms: number;
  /** One entry per attempt; null means it failed or timed out. */
  samples_ms: (number | null)[];
  stats: ProbeStats | null;
};

export function formatMs(ms: number): string {
  return ms.toFixed(ms < 100 ? 1 : 0);
}
