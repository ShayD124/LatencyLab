import { PingPanel } from "./components/PingPanel";
import { ProbePanel } from "./components/ProbePanel";

export default function Home() {
  return (
    <main className="mx-auto w-full max-w-5xl flex-1 px-4 py-10 sm:px-6">
      <header className="mb-8">
        <h1 className="text-2xl font-semibold tracking-tight">LatencyLab</h1>
        <p className="mt-1 text-ink-2">
          See how long the network takes: from you to the server, and from the
          server to the rest of the internet.
        </p>
      </header>

      <div className="grid items-start gap-6 lg:grid-cols-2">
        <PingPanel />
        <ProbePanel />
      </div>
    </main>
  );
}
