# Matrix Multiplication (single-node baseline)
# Packages: numpy
def user_fn(n):
    import numpy as np
    a = np.random.rand(n, n)
    b = np.random.rand(n, n)
    return {"norm": float(np.linalg.norm(a @ b)), "n": n}
