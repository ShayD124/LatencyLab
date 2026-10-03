"use client";

import { useState } from "react";
import { API_URL, formatMs, type ProbeResult } from "@/app/lib/api";
import { ErrorNote, StatTile } from "./StatTile";

const EXAMPLES = ["github.com", "cloudflare.com", "wikipedia.org"];

/** Accepts a pasted URL or a bare hostname and returns just the hostname. */
function toHostname(input: string): string {
  const trimmed = input.trim();
  try {
    return new URL(trimmed.includes("://") ? trimmed : `https://${trimmed}`)
      .hostname;
  } catch {
    return trimmed;
  }
}

export function ProbePanel() {
  const [host, setHost] = useState("github.com");
  const [port, setPort] = useState(443);
  const [sampleCount, setSampleCount] = useState(5);
  const [loading, setLoading] = useState(false);
  const [result, setResult] = useState<ProbeResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const run = async (target: string) => {
    const hostname = toHostname(target);
    setHost(hostname);
    setLoading(true);
    setError(null);
    try {
      const res = await fetch(`${API_URL}/api/probe`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ host: hostname, port, samples: sampleCount }),
      });
      const body = await res.json().catch(() => null);
      if (res.ok && body) {
        setResult(body);
      } else {
        setError(body?.error ?? `The API returned an error (${res.status}).`);
      }
    } catch {
      setError(`Can't reach the API at ${API_URL}.`);
    } finally {
      setLoading(false);
    }
  };

  const fieldClass =
    "rounded-md border border-hairline bg-page px-3 py-1.5 text-sm";

  return (
    <section className="rounded-xl border border-hairline bg-surface p-5">
      <h2 className="font-semibold">API → any website</h2>
      <p className="mt-0.5 text-sm text-ink-2">
        The server opens TCP connections to a host and times each one. A
        connection takes about one network round trip.
      </p>

      <form
        className="mt-5 flex flex-wrap items-end gap-3"
        onSubmit={(e) => {
          e.preventDefault();
          run(host);
        }}
      >
        <label className="flex min-w-40 flex-1 flex-col gap-1 text-xs text-ink-2">
          Host
          <input
            className={`${fieldClass} text-ink`}
            value={host}
            onChange={(e) => setHost(e.target.value)}
            placeholder="example.com"
            autoCapitalize="none"
            spellCheck={false}
            required
          />
        </label>
        <label className="flex flex-col gap-1 text-xs text-ink-2">
          Port
          <select
            className={`${fieldClass} text-ink`}
            value={port}
            onChange={(e) => setPort(Number(e.target.value))}
          >
            <option value={443}>443 (HTTPS)</option>
            <option value={80}>80 (HTTP)</option>
          </select>
        </label>
        <label className="flex flex-col gap-1 text-xs text-ink-2">
          Samples
          <select
            className={`${fieldClass} text-ink`}
            value={sampleCount}
            onChange={(e) => setSampleCount(Number(e.target.value))}
          >
            {[3, 5, 10].map((n) => (
              <option key={n} value={n}>
                {n}
              </option>
            ))}
          </select>
        </label>
        <button
          type="submit"
          disabled={loading}
          className="rounded-md bg-ink px-4 py-1.5 text-sm font-medium text-page disabled:opacity-50"
        >
          {loading ? "Measuring…" : "Measure"}
        </button>
      </form>

      <div className="mt-3 flex flex-wrap items-center gap-2 text-xs text-ink-2">
        Try
        {EXAMPLES.map((example) => (
          <button
            key={example}
            type="button"
            disabled={loading}
            onClick={() => run(example)}
            className="rounded-full border border-hairline px-2.5 py-1 hover:bg-page disabled:opacity-50"
          >
            {example}
          </button>
        ))}
      </div>

      {error && (
        <div className="mt-5">
          <ErrorNote>{error}</ErrorNote>
        </div>
      )}

      {result && (
        <div
          className={`mt-6 transition-opacity ${loading ? "opacity-50" : ""}`}
        >
          <ProbeResultView result={result} />
        </div>
      )}
    </section>
  );
}

function ProbeResultView({ result }: { result: ProbeResult }) {
  const { stats, samples_ms: samples } = result;
  const failed = samples.filter((ms) => ms === null).length;
  const max = stats?.max_ms ?? 1;

  return (
    <>
      <p className="text-sm">
        <span className="font-semibold">{result.host}</span>
        <span className="text-ink-2">
          {" "}
          at {result.ip}, port {result.port}
        </span>
      </p>

      <dl className="mt-4 grid grid-cols-3 gap-4">
        <StatTile label="Median" value={stats ? formatMs(stats.p50_ms) : "–"} unit="ms" />
        <StatTile label="Average" value={stats ? formatMs(stats.avg_ms) : "–"} unit="ms" />
        <StatTile label="DNS lookup" value={formatMs(result.dns_ms)} unit="ms" />
        <StatTile label="Fastest" value={stats ? formatMs(stats.min_ms) : "–"} unit="ms" />
        <StatTile label="Slowest" value={stats ? formatMs(stats.max_ms) : "–"} unit="ms" />
        <StatTile label="Failed" value={String(failed)} unit={`of ${samples.length}`} />
      </dl>

      <h3 className="mt-6 mb-3 text-xs text-ink-2">
        Connect time per sample, in ms
      </h3>
      <ol className="flex flex-col gap-1.5">
        {samples.map((ms, i) => (
          <li
            key={i}
            className="group flex items-center gap-3 text-xs tabular-nums"
            title={
              ms === null
                ? `Sample ${i + 1}: no response`
                : `Sample ${i + 1}: ${ms} ms`
            }
          >
            <span className="w-5 shrink-0 text-right text-muted">{i + 1}</span>
            <div className="flex flex-1 items-center gap-2 border-l border-axis py-0.5">
              {ms === null ? (
                <span className="pl-2 text-ink-2">No response</span>
              ) : (
                <>
                  <div
                    className="h-3 min-w-0.5 rounded-r bg-series group-hover:brightness-125"
                    style={{ width: `calc((100% - 4rem) * ${ms / max})` }}
                  />
                  <span className="text-ink">{formatMs(ms)}</span>
                </>
              )}
            </div>
          </li>
        ))}
      </ol>

      {!stats && (
        <div className="mt-4">
          <ErrorNote>
            Every attempt failed. The host may be blocking connections on port{" "}
            {result.port}.
          </ErrorNote>
        </div>
      )}
    </>
  );
}
