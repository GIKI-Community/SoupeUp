# Matrix Multiplication (distributed via Ray)
# Packages: numpy
n = ARGS[0]
import numpy as np
import ray


@ray.remote
def mul_block(_i, size=n):
    a = np.random.rand(size, size)
    b = np.random.rand(size, size)
    return float(np.linalg.norm(a @ b))


futures = [mul_block.remote(i) for i in range(workers * 2)]
norms = ray.get(futures)
result = {"blocks": len(norms), "meanNorm": float(sum(norms) / len(norms)), "n": n}
