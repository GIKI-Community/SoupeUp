# Mandelbrot Renderer (distributed via Ray)
# Packages: numpy
width, height, max_iter = ARGS
import numpy as np
import ray


@ray.remote
def row(y, w=width, h=height, mi=max_iter):
    row_out = []
    for x in range(w):
        c = complex(-2.0 + 2.7 * x / w, -1.2 + 2.4 * y / h)
        z = 0j
        n = 0
        while abs(z) <= 2 and n < mi:
            z = z * z + c
            n += 1
        row_out.append(n)
    return row_out


futures = [row.remote(y) for y in range(height)]
rows = ray.get(futures)
arr = np.array(rows, dtype=np.uint16)
result = {
    "width": width,
    "height": height,
    "maxIter": max_iter,
    "meanIterations": float(arr.mean()),
    "maxReached": int((arr == max_iter).sum()),
}
