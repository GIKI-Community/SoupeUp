# Mandelbrot Renderer (single-node baseline)
# Packages: numpy
def user_fn(width, height, max_iter):
    import numpy as np

    def row(y):
        row_out = []
        for x in range(width):
            c = complex(-2.0 + 2.7 * x / width, -1.2 + 2.4 * y / height)
            z = 0j
            n = 0
            while abs(z) <= 2 and n < max_iter:
                z = z * z + c
                n += 1
            row_out.append(n)
        return row_out

    rows = [row(y) for y in range(height)]
    arr = np.array(rows, dtype=np.uint16)
    return {
        "width": width,
        "height": height,
        "maxIter": max_iter,
        "meanIterations": float(arr.mean()),
        "maxReached": int((arr == max_iter).sum()),
    }
