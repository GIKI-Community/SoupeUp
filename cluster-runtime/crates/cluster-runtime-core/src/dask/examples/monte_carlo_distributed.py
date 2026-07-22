# Monte Carlo π Estimation (distributed)
# Packages: (stdlib only)
samples = ARGS[0]
import random


def chunk(n):
    inside = 0
    for _ in range(n):
        x = random.random()
        y = random.random()
        if x * x + y * y <= 1.0:
            inside += 1
    return inside


workers = max(1, len(client.scheduler_info().get("workers", {})))
chunk_size = samples // workers
sizes = [chunk_size] * workers
sizes[-1] += samples - chunk_size * workers
futures = client.map(chunk, sizes)
inside = sum(client.gather(futures))
pi = 4.0 * inside / samples
result = {"pi": pi, "samples": samples, "workers": workers}
