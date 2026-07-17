# Mandelbrot Renderer (single-node baseline)
# Packages: numpy
def user_fn(width, height, max_iter):
    import numpy as np
    arr = np.zeros((height, width), dtype=np.uint16)
    for y in range(height):
        for x in range(width):
            c = complex(-2.0 + 2.7 * x / width, -1.2 + 2.4 * y / height)
            z = 0j
            n = 0
            while abs(z) <= 2 and n < max_iter:
                z = z * z + c
                n += 1
            arr[y, x] = n
    return {
        "width": width,
        "height": height,
        "meanIterations": float(arr.mean()),
    }
