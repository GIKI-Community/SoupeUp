def main():
    """
    Research-style workflow:
      - discretize a 2D nonlinear PDE (Gray–Scott)
      - sweep a (F, k) parameter grid (ensemble)
      - evolve each case in time on Dask workers
      - return scientific diagnostics (pattern energy, spectra, rankings)

    Needs: numpy (usually present with Dask). Tune GRID / STEPS / ensemble size for load.
    """
    from distributed import get_client
    import itertools
    import math
    import socket
    import time

    try:
        import numpy as np
    except ImportError as e:
        raise RuntimeError("numpy is required for this job") from e

    # ---- knobs (turn these up for a real workout) ----
    GRID = 192          # 192² or try 256 / 384
    STEPS = 8_000       # time steps per (F, k) run
    DU, DV = 0.16, 0.08
    DT = 1.0
    SEED = 7

    # Classic Gray–Scott interesting band; denser grid = more work
    F_vals = np.linspace(0.018, 0.060, 10)
    K_vals = np.linspace(0.045, 0.065, 10)
    # 10×10 = 100 independent PDE solves

    def laplacian(z):
        # 5-point stencil, periodic BC
        return (
            -4.0 * z
            + np.roll(z, 1, 0)
            + np.roll(z, -1, 0)
            + np.roll(z, 1, 1)
            + np.roll(z, -1, 1)
        )

    def simulate_one(payload):
        F, k, n, steps, du, dv, dt, seed = payload
        rng = np.random.default_rng(seed + int(F * 1e6) + int(k * 1e6))

        U = np.ones((n, n), dtype=np.float64)
        V = np.zeros((n, n), dtype=np.float64)

        # localized perturbation (typical GS initial condition)
        r = n // 8
        c = n // 2
        U[c - r : c + r, c - r : c + r] = 0.50
        V[c - r : c + r, c - r : c + r] = 0.25
        U += 0.02 * rng.standard_normal((n, n))
        V += 0.02 * rng.standard_normal((n, n))
        np.clip(U, 0.0, 1.0, out=U)
        np.clip(V, 0.0, 1.0, out=V)

        # evolve
        for _ in range(steps):
            Lu = laplacian(U)
            Lv = laplacian(V)
            uvv = U * V * V
            U += dt * (du * Lu - uvv + F * (1.0 - U))
            V += dt * (dv * Lv + uvv - (F + k) * V)

        # diagnostics a researcher might log
        v_mean = float(V.mean())
        v_std = float(V.std())
        # "pattern energy" in Fourier space (high-k content ⇒ spots/stripes)
        Vh = np.fft.rfft2(V - v_mean)
        power = np.abs(Vh) ** 2
        # ignore DC
        power[0, 0] = 0.0
        total_power = float(power.sum()) + 1e-30
        # radial peak frequency (very rough morphology proxy)
        ky = np.fft.fftfreq(n)[:, None]
        kx = np.fft.rfftfreq(n)[None, :]
        freq = np.sqrt(kx * kx + ky * ky)
        peak_freq = float(freq.ravel()[np.argmax(power.ravel())])

        # spot-likeness: fraction of cells above mean+std
        spot_frac = float((V > (v_mean + v_std)).mean())

        score = v_std * math.log1p(total_power)  # crude "interesting pattern" score

        return {
            "F": float(F),
            "k": float(k),
            "v_mean": v_mean,
            "v_std": v_std,
            "spot_frac": spot_frac,
            "spectral_power": total_power,
            "peak_freq": peak_freq,
            "score": float(score),
            "host": socket.gethostname(),
        }

    client = get_client()
    worker_count = len(client.scheduler_info().get("workers", {}))

    jobs = [
        (float(F), float(k), GRID, STEPS, DU, DV, DT, SEED)
        for F, k in itertools.product(F_vals, K_vals)
    ]

    t0 = time.time()
    futures = client.map(simulate_one, jobs, pure=False)
    results = client.gather(futures)
    elapsed = time.time() - t0

    results_sorted = sorted(results, key=lambda r: r["score"], reverse=True)
    top = results_sorted[:8]

    # host participation (are remote workers actually used?)
    from collections import Counter
    host_counts = dict(Counter(r["host"] for r in results))

    return {
        "experiment": "Gray-Scott parameter ensemble (finite-difference PDE)",
        "grid": GRID,
        "steps_per_run": STEPS,
        "ensemble_size": len(jobs),
        "dask_workers": worker_count,
        "wall_seconds": round(elapsed, 2),
        "host_task_counts": host_counts,
        "top_patterns": top,
        "note": "Raise GRID→256 and STEPS→20000 for a serious overnight-style load.",
    }