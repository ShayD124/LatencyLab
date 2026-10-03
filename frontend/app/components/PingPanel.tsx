"use client";

import { useEffect, useState } from "react";
import { API_URL, formatMs } from "@/app/lib/api";
import { LatencyChart, type PingSample } from "./LatencyChart";
import { ErrorNote, StatTile } from "./StatTile";

const SLOTS = 60;
const INTERVAL_MS = 1000;
const TIMEOUT_MS = 5000;

/** Round-trip time of one request to the API in ms, or null if it failed. */
async function pingOnce(): Promise<number | null> {
  const start = performance.now();
  try {
    const res = await fetch(`${API_URL}/api/ping`, {
      cache: "no-store",
      signal: AbortSignal.timeout(TIMEOUT_MS),
    });
    if (!res.ok) return null;
    await res.arrayBuffer();
    return performance.now() - start;
  } catch {
    return null;
  }
}

export function PingPanel() {
  const [samples, setSamples] = useState<PingSample[]>([]);
  const [running, setRunning] = useState(true);

  useEffect(() => {
    if (!running) return;
    let cancelled = false;
    let timer: ReturnType<typeof setTimeout> | undefined;

    const tick = async () => {
      // Background tabs throttle timers, which would show up as fake latency.
      if (!document.hidden) {
        const ms = await pingOnce();
        if (cancelled) return;
        setSamples((prev) => [...prev, { at: Date.now(), ms }].slice(-SLOTS));
      }
      timer = setTimeout(tick, INTERVAL_MS);
    };

    // The first request also pays for the TCP and TLS handshakes, so it is not recorded.
    pingOnce().then(() => {
      if (!cancelled) tick();
    });

    return () => {
      cancelled = true;
      clearTimeout(timer);
    };
  }, [running]);

  const ok = samples.flatMap((s) => (s.ms === null ? [] : [s.ms]));
  const failed = samples.length - ok.length;
  const last = samples.length ? samples[samples.length - 1] : null;
  const unreachable = samples.length > 0 && ok.length === 0;
  const stat = (value: number | undefined) =>
    value === undefined ? "–" : formatMs(value);

  return (
    <section className="rounded-xl border border-hairline bg-surface p-5">
      <div className="flex items-start justify-between gap-4">
        <div>
          <h2 className="font-semibold">Your browser → API</h2>
          <p className="mt-0.5 text-sm text-ink-2">
            Round-trip time of a tiny request, once a second.
          </p>
        </div>
        <button
          type="button"
          onClick={() => setRunning((r) => !r)}
          className="rounded-md border border-hairline px-3 py-1.5 text-sm hover:bg-page"
        >
          {running ? "Pause" : "Resume"}
        </button>
      </div>

      <p className="mt-5 text-5xl font-semibold">
        {last?.ms != null ? formatMs(last.ms) : "–"}
        <span className="ml-2 text-base font-normal text-ink-2">ms</span>
      </p>

      <dl className="mt-4 grid grid-cols-4 gap-4">
        <StatTile
          label="Average"
          value={stat(ok.length ? ok.reduce((a, b) => a + b, 0) / ok.length : undefined)}
          unit="ms"
        />
        <StatTile label="Fastest" value={stat(ok.length ? Math.min(...ok) : undefined)} unit="ms" />
        <StatTile label="Slowest" value={stat(ok.length ? Math.max(...ok) : undefined)} unit="ms" />
        <StatTile label="Failed" value={String(failed)} unit={`of ${samples.length}`} />
      </dl>

      <div className="mt-6">
        <h3 className="mb-3 text-xs text-ink-2">
          Latency in ms, last {SLOTS} pings
        </h3>
        <LatencyChart samples={samples} slots={SLOTS} />
      </div>

      {unreachable && (
        <div className="mt-4">
          <ErrorNote>
            Can&apos;t reach the API at {API_URL}. On free hosting it sleeps when
            idle and takes about a minute to wake up. If this message stays,
            check that the backend is running and that this site&apos;s address
            is in its ALLOWED_ORIGINS.
          </ErrorNote>
        </div>
      )}

      <details className="mt-4 text-sm">
        <summary className="cursor-pointer text-ink-2">Show as table</summary>
        <div className="mt-2 max-h-48 overflow-y-auto">
          <table className="w-full text-left tabular-nums">
            <thead className="text-xs text-ink-2">
              <tr>
                <th className="py-1 font-normal">Time</th>
                <th className="py-1 font-normal">Latency</th>
              </tr>
            </thead>
            <tbody>
              {[...samples].reverse().map((s) => (
                <tr key={s.at} className="border-t border-grid">
                  <td className="py-1">{new Date(s.at).toLocaleTimeString()}</td>
                  <td className="py-1">
                    {s.ms === null ? "Failed" : `${formatMs(s.ms)} ms`}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </details>
    </section>
  );
}
