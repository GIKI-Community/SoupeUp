def main():
    """Monte Carlo π — parallel chunks across the Dask cluster."""
    from distributed import get_client
    import math
    import socket
    import time

    # Tune these up/down for load
    TOTAL_SAMPLES = 50_000_000   # 50 million
    N_CHUNKS = 32

    def count_inside(n: int) -> int:
        # Independent RNG stream per task (no shared state)
        import random
        rng = random.Random()
        inside = 0
        for _ in range(n):
            x = rng.random()
            y = rng.random()
            if x * x + y * y <= 1.0:
                inside += 1
        return inside

    client = get_client()
    workers = list(client.scheduler_info().get("workers", {}))
    per_chunk = TOTAL_SAMPLES // N_CHUNKS

    t0 = time.time()
    futures = client.map(count_inside, [per_chunk] * N_CHUNKS)
    insides = client.gather(futures)
    elapsed = time.time() - t0

    total_inside = sum(insides)
    pi_est = 4.0 * total_inside / (per_chunk * N_CHUNKS)

    return {
        "pi_estimate": pi_est,
        "error_vs_math_pi": abs(pi_est - math.pi),
        "samples": per_chunk * N_CHUNKS,
        "chunks": N_CHUNKS,
        "workers_seen": len(workers),
        "seconds": round(elapsed, 3),
        "submitted_from": socket.gethostname(),
    }