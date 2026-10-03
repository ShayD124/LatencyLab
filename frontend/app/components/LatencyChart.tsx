"use client";

import { useState } from "react";
import { formatMs } from "@/app/lib/api";

export type PingSample = { at: number; ms: number | null };

/** Rounds up to 1, 2 or 5 times a power of ten so axis ticks land on clean numbers. */
function niceMax(value: number): number {
  const pow = 10 ** Math.floor(Math.log10(value));
  const n = value / pow;
  return (n <= 1 ? 1 : n <= 2 ? 2 : n <= 5 ? 5 : 10) * pow;
}

/**
 * Line chart of the most recent pings, newest at the right edge.
 * The SVG is stretched to fit, so anything that must stay round or readable
 * (dot, crosshair, labels) is drawn as HTML on top of it.
 */
export function LatencyChart({
  samples,
  slots,
}: {
  samples: PingSample[];
  slots: number;
}) {
  const [hover, setHover] = useState<number | null>(null);

  const count = samples.length;
  const okValues = samples.flatMap((s) => (s.ms === null ? [] : [s.ms]));
  const max = niceMax(Math.max(1, ...okValues));
  const ticks = [max, max / 2, 0];

  const x = (i: number) => ((slots - count + i) / (slots - 1)) * 100;
  const y = (ms: number) => 100 - (ms / max) * 100;

  // Failed pings break the line into separate runs.
  const runs: { i: number; ms: number }[][] = [];
  let run: { i: number; ms: number }[] = [];
  samples.forEach((s, i) => {
    if (s.ms === null) {
      if (run.length) runs.push(run);
      run = [];
    } else {
      run.push({ i, ms: s.ms });
    }
  });
  if (run.length) runs.push(run);

  const points = (r: { i: number; ms: number }[]) =>
    r.map((p) => `${x(p.i)},${y(p.ms)}`).join(" L");
  const linePath = runs.map((r) => `M${points(r)}`).join(" ");
  const areaPath = runs
    .map((r) => `M${x(r[0].i)},100 L${points(r)} L${x(r[r.length - 1].i)},100 Z`)
    .join(" ");

  const last = count ? samples[count - 1] : null;
  const hovered = hover !== null && hover < count ? samples[hover] : null;
  const hoveredX = hovered && hover !== null ? x(hover) : 0;

  const indexAt = (clientX: number, rect: DOMRect) => {
    if (!count) return null;
    const slot = Math.round(((clientX - rect.left) / rect.width) * (slots - 1));
    return Math.min(count - 1, Math.max(0, slot - (slots - count)));
  };

  const onKeyDown = (e: React.KeyboardEvent) => {
    if (!count || (e.key !== "ArrowLeft" && e.key !== "ArrowRight")) return;
    e.preventDefault();
    const from = hover ?? count - 1;
    const step = e.key === "ArrowLeft" ? -1 : 1;
    setHover(Math.min(count - 1, Math.max(0, from + step)));
  };

  return (
    <div>
      <div className="flex h-40">
        <div className="relative w-9 shrink-0 text-right text-xs tabular-nums text-muted">
          {ticks.map((t) => (
            <span
              key={t}
              className="absolute right-2 -translate-y-1/2"
              style={{ top: `${y(t)}%` }}
            >
              {t}
            </span>
          ))}
        </div>

        <div
          className="relative flex-1 cursor-crosshair rounded-sm outline-offset-4 focus-visible:outline-2 focus-visible:outline-series"
          role="group"
          tabIndex={0}
          aria-label={`Latency of the last ${count} pings. Use left and right arrow keys to read values.`}
          onPointerMove={(e) =>
            setHover(indexAt(e.clientX, e.currentTarget.getBoundingClientRect()))
          }
          onPointerLeave={() => setHover(null)}
          onFocus={() => setHover(count ? count - 1 : null)}
          onBlur={() => setHover(null)}
          onKeyDown={onKeyDown}
        >
          {ticks.map((t) => (
            <div
              key={t}
              className={`absolute inset-x-0 border-t ${t === 0 ? "border-axis" : "border-grid"}`}
              style={{ top: `${y(t)}%` }}
            />
          ))}

          <svg
            className="absolute inset-0 size-full overflow-visible"
            viewBox="0 0 100 100"
            preserveAspectRatio="none"
            aria-hidden="true"
          >
            <path d={areaPath} className="fill-series opacity-10" />
            <path
              d={linePath}
              className="fill-none stroke-series"
              strokeWidth={2}
              strokeLinejoin="round"
              strokeLinecap="round"
              vectorEffect="non-scaling-stroke"
            />
          </svg>

          {hovered && (
            <div
              className="absolute inset-y-0 w-px bg-axis"
              style={{ left: `${hoveredX}%` }}
            />
          )}

          {last && last.ms !== null && (
            <Dot left={x(count - 1)} top={y(last.ms)} />
          )}
          {hovered && hovered.ms !== null && hover !== count - 1 && (
            <Dot left={hoveredX} top={y(hovered.ms)} />
          )}

          {hovered && (
            <div
              className="pointer-events-none absolute top-0 z-10 whitespace-nowrap rounded-md border border-hairline bg-surface px-2 py-1 text-xs shadow-sm"
              style={
                hoveredX > 50
                  ? { right: `calc(${100 - hoveredX}% + 8px)` }
                  : { left: `calc(${hoveredX}% + 8px)` }
              }
            >
              <div className="font-semibold text-ink">
                {hovered.ms === null ? "Failed" : `${formatMs(hovered.ms)} ms`}
              </div>
              <div className="text-ink-2">
                {new Date(hovered.at).toLocaleTimeString()}
              </div>
            </div>
          )}
        </div>
      </div>

      <div className="mt-1.5 ml-9 flex justify-between text-xs text-muted">
        <span>{slots} pings ago</span>
        <span>now</span>
      </div>
    </div>
  );
}

function Dot({ left, top }: { left: number; top: number }) {
  return (
    <div
      className="absolute size-2 -translate-x-1/2 -translate-y-1/2 rounded-full bg-series ring-2 ring-surface"
      style={{ left: `${left}%`, top: `${top}%` }}
    />
  );
}
