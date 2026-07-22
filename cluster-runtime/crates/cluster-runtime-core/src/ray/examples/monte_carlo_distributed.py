# Monte Carlo π Estimation (distributed via Ray)
# Packages: (stdlib only)
samples = ARGS[0]
import random
import ray


@ray.remote
def chunk(n):
    inside = 0
    for _ in range(n):
        x = random.random()
        y = random.random()
        if x * x + y * y <= 1.0:
            inside += 1
    return inside


chunk_size = samples // workers
sizes = [chunk_size] * workers
sizes[-1] += samples - chunk_size * workers
futures = [chunk.remote(s) for s in sizes]
inside = sum(ray.get(futures))
pi = 4.0 * inside / samples
result = {"pi": pi, "samples": samples, "workers": workers}
