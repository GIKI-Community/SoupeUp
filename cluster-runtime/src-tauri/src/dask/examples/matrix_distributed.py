# Matrix Multiplication (distributed)
# Packages: numpy
n = ARGS[0]
import numpy as np


def mul_block(_i, size=n):
    a = np.random.rand(size, size)
    b = np.random.rand(size, size)
    return float(np.linalg.norm(a @ b))


workers = max(1, len(client.scheduler_info().get("workers", {})))
futures = client.map(mul_block, list(range(workers * 2)))
norms = client.gather(futures)
result = {"blocks": len(norms), "meanNorm": float(sum(norms) / len(norms)), "n": n}
