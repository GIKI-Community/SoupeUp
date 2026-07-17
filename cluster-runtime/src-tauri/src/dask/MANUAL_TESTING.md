# Dask Scheduler Plugin — Manual Test Plan

All testing is manual. No automated tests are required for this milestone.

## Prerequisites

```powershell
cd cluster-runtime

# 1. Ensure bundled / system Python is available
.\scripts\Setup-PythonRuntime.ps1

# 2. Install frontend deps (once)
npm install

# 3. Run the app
npm run tauri dev
```

Wait until **Plugins** shows:

* Python Runtime → `running`
* Dask Scheduler → `running`

First launch may take a few minutes while Dask packages install into the managed venv.

---

## Single-machine smoke test

1. Open **Cluster**.
2. Click **Ensure Packages** (optional if init already succeeded).
3. Click **Start Scheduler** → status becomes `running`; address like `tcp://127.0.0.1:8786`.
4. Set worker address to `tcp://127.0.0.1:8786` → **Start Worker**.
5. Confirm the workers table shows at least one worker and cores/memory populate.
6. Run **Monte Carlo π Estimation** → success, execution time, workers used.
7. Open dashboard tabs (or **Open** in browser) → Task Stream / Workers visible.
8. **Stop Worker** then **Stop Scheduler** → both `stopped`.

---

## Multi-node test (success criteria)

### Machine A (scheduler)

1. Install + launch Cluster Runtime.
2. **Settings → Dask Scheduler**: set Scheduler Address to `tcp://<A-LAN-IP>:8786`.
3. **Cluster → Start Scheduler**.
4. Optionally also start a local worker.

### Machines B and C (workers)

1. Launch Cluster Runtime.
2. Paste `tcp://<A-LAN-IP>:8786` into Scheduler address.
3. **Start Worker**.

### Verify

1. On A, Cluster page lists workers from A/B/C.
2. Run **Mandelbrot Renderer**.
3. Watch worker CPU / dashboard Task Stream during the run.
4. Note execution time + speedup vs baseline.
5. Stop all workers, then stop scheduler.

---

## Firewall notes (Windows)

```powershell
# Allow inbound Dask scheduler port (run as Admin if needed)
New-NetFirewallRule -DisplayName "Dask Scheduler 8786" -Direction Inbound -Protocol TCP -LocalPort 8786 -Action Allow
New-NetFirewallRule -DisplayName "Dask Dashboard 8787" -Direction Inbound -Protocol TCP -LocalPort 8787 -Action Allow
```

---

## Settings checks

**Settings → Dask Scheduler**

* Change dashboard port → restart scheduler → dashboard URL updates.
* Change worker name / threads → restart worker → reflected in cluster table.

**Metrics**

* With workers connected, Worker CPU / Memory / Load cards update.

---

## Failure injection

| Action | Expected |
|---|---|
| Stop scheduler while worker running | Worker eventually errors / disconnects; UI does not crash |
| Start worker with bad address | Error message; status `error` or failed connect |
| Run example with no scheduler | Clear client/job error |
| Disable network mid-job | Job fails gracefully with error string |

---

## Example jobs checklist

- [ ] Mandelbrot Renderer
- [ ] Monte Carlo π Estimation
- [ ] Matrix Multiplication
- [ ] Prime Number Search
- [ ] Image Blur
- [ ] Word Count
