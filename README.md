# LatencyLab

A small web app that measures network latency in two directions:

- **Your browser → API.** The page sends a tiny request to the server once a second and times the round trip.
- **API → any website.** The server opens TCP connections to a host you name and times each one.

It is split the way production apps usually are:

| Part | Folder | Built with | Hosted on |
|---|---|---|---|
| Frontend | `frontend/` | Next.js | Vercel |
| Backend API | `backend/` | Rust (Axum), in Docker | Render (free), or your own server behind Caddy |

## Run it on your computer

You need Node.js and Rust. Open two terminals.

**Terminal 1, the API:**

```
cd backend
cargo run
```

It listens on http://localhost:8080. Check it with http://localhost:8080/health.

**Terminal 2, the website:**

```
cd frontend
npm install
npm run dev
```

Open http://localhost:3000.

Run the backend tests with `cargo test` inside `backend/`.

## How the pieces talk to each other

The page in the browser calls the API directly, and the two live on different addresses. Two settings connect them:

| Setting | Where | What it does |
|---|---|---|
| `NEXT_PUBLIC_API_URL` | Frontend (Vercel) | Tells the page where the API is. |
| `ALLOWED_ORIGINS` | Backend (Render dashboard, or `.env` on your own server) | Tells the API which websites may call it. This is CORS. |

If these do not match your real addresses, the page loads but shows "Can't reach the API".

## Deploy it for free (Render + Vercel)

The backend runs on Render's free plan and the website on Vercel. Neither needs a payment card. On the free plan the API sleeps after about 15 minutes without visitors, and the first visit after that waits up to a minute while it wakes.

### 1. Backend on Render

1. Sign up at https://render.com with your GitHub account.
2. Click **New → Blueprint** and pick the `LatencyLab` repository. Render reads `render.yaml` from it.
3. When asked for `ALLOWED_ORIGINS`, enter `http://localhost:3000` for now.
4. Click **Apply**. The first build takes several minutes.
5. Open the `latencylab-api` service and copy its address, which looks like `https://latencylab-api.onrender.com`. Adding `/health` to it in a browser should show `ok`.

### 2. Website on Vercel

1. At https://vercel.com/new, import the `LatencyLab` repository.
2. Set **Root Directory** to `frontend`.
3. Under **Environment Variables**, add `NEXT_PUBLIC_API_URL` with the Render address from step 1 (no slash at the end).
4. Deploy, then copy the website's address, which looks like `https://latency-lab.vercel.app`.

### 3. Allow the website to call the API

1. In Render, open `latencylab-api` → **Environment**.
2. Change `ALLOWED_ORIGINS` to the Vercel address from step 2 (no slash at the end).
3. Save. Render restarts the API on its own.

Open the website. The left panel should show numbers within a few seconds, or within a minute if the API was asleep.

### 4. Use your own domain (optional)

1. In Vercel, open the project's **Settings → Domains** and add your domain. Vercel shows the DNS records to create where you manage the domain.
2. In Render, add the new address to `ALLOWED_ORIGINS`, separated by commas: `https://latency-lab.vercel.app,https://example.com,https://www.example.com`.

### Updating

Push to GitHub. Vercel and Render both redeploy on their own.

## Alternative: run the backend on your own server

This replaces Render with a server you rent, running Docker and Caddy. It costs a few euros a month and the API never sleeps.

The steps below use `example.com`. Replace it with your own domain everywhere. The website will be at `example.com` and the API at `api.example.com`.

### 1. Put the code on GitHub

Create a new empty repository at https://github.com/new (no README, no .gitignore). Then, in this folder:

```
git add .
git commit -m "Initial commit"
git remote add origin https://github.com/YOUR-USERNAME/LatencyLab.git
git push -u origin main
```

### 2. Create the server

1. At Hetzner Cloud, create a server with **Ubuntu 24.04**. The smallest shared plan is enough.
2. Add your SSH key when asked, so you can log in without a password.
3. Note the server's IPv4 address.
4. If you add a Hetzner firewall, allow inbound TCP 22, 80 and 443, and UDP 443.

### 3. Point the API address at the server

Where you manage your domain's DNS, add:

| Type | Name | Value |
|---|---|---|
| A | `api` | the server's IPv4 address |

Do this before step 4. Caddy can only get an HTTPS certificate once the name points at the server.

### 4. Start the API on the server

Log in and install Docker:

```
ssh root@YOUR-SERVER-IP
curl -fsSL https://get.docker.com | sh
```

Get the code and configure it:

```
git clone https://github.com/YOUR-USERNAME/LatencyLab.git
cd LatencyLab
cp .env.example .env
nano .env
```

In `.env`, set your real addresses:

```
API_DOMAIN=api.example.com
ALLOWED_ORIGINS=https://example.com,https://www.example.com
```

Build and start:

```
docker compose up -d --build
```

The first build takes a few minutes. Then check from any computer:

```
curl https://api.example.com/health
```

It should print `ok`.

### 5. Put the website on Vercel

1. At https://vercel.com/new, import the GitHub repository.
2. Set **Root Directory** to `frontend`.
3. Under **Environment Variables**, add `NEXT_PUBLIC_API_URL` with the value `https://api.example.com`.
4. Deploy.
5. In the project's **Settings → Domains**, add `example.com`. Vercel shows the DNS records to create; add them where you manage your domain.

`NEXT_PUBLIC_API_URL` is baked in when the site is built. If you change it, redeploy.

### 6. Check it

Open `https://example.com`. The left panel should start showing numbers within a few seconds, and "Measure" on the right should return results.

### Updating your own server

- **Frontend:** push to GitHub. Vercel redeploys on its own.
- **Backend:** on the server, run:

  ```
  cd LatencyLab
  git pull
  docker compose up -d --build
  ```

## When something is wrong

| What you see | Likely cause | What to do |
|---|---|---|
| Page shows "Can't reach the API" for up to a minute, then works | The free Render service was asleep | Nothing, this is normal on the free plan |
| Page keeps showing "Can't reach the API" | `ALLOWED_ORIGINS` does not include the exact address in your browser's address bar | Fix it in Render → Environment (or in `.env` on your own server, then `docker compose up -d`) |
| Same, only on Vercel preview links | Preview deployments have their own `*.vercel.app` addresses | Expected. Only the addresses in `ALLOWED_ORIGINS` work |
| `curl https://api.example.com/health` fails with a certificate error | DNS does not point at the server yet, or ports 80/443 are blocked | Check the A record and firewall, then `docker compose logs caddy` |
| "Too many probes" | The API allows 10 measurements a minute per visitor | Wait a minute |

Useful commands on your own server:

```
docker compose ps          # what is running
docker compose logs api    # API logs
docker compose logs caddy  # HTTPS and proxy logs
```

## API

| Method | Path | Purpose |
|---|---|---|
| GET | `/health` | Returns `ok`. |
| GET | `/api/ping` | Tiny response the browser times. |
| POST | `/api/probe` | Body `{"host": "github.com", "port": 443, "samples": 5}`. Returns connect times and statistics. |

The probe only connects to public addresses on ports 80 and 443, takes at most 10 samples, and is rate limited, so the server cannot be used to scan private networks.
