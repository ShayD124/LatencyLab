export function StatTile({
  label,
  value,
  unit,
}: {
  label: string;
  value: string;
  unit?: string;
}) {
  return (
    <div>
      <dt className="text-xs text-ink-2">{label}</dt>
      <dd className="mt-0.5 text-lg font-semibold">
        {value}
        {unit && (
          <span className="ml-1 text-xs font-normal text-ink-2">{unit}</span>
        )}
      </dd>
    </div>
  );
}

export function ErrorNote({ children }: { children: React.ReactNode }) {
  return (
    <p role="alert" className="flex items-start gap-2 text-sm text-ink">
      <svg
        aria-hidden="true"
        viewBox="0 0 16 16"
        className="mt-0.5 size-4 shrink-0 fill-critical"
      >
        <path d="M8 1a7 7 0 1 0 0 14A7 7 0 0 0 8 1Zm-.75 3.5h1.5v5h-1.5v-5Zm.75 8a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z" />
      </svg>
      <span>{children}</span>
    </p>
  );
}
