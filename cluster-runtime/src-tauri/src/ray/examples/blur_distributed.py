# Image Blur (distributed via Ray)
# Packages: numpy
width, height = ARGS
import numpy as np
import ray


@ray.remote
def blur_row(y, w=width):
    row = np.sin(np.linspace(0, 8, w) + y * 0.05)
    kernel = np.array([0.25, 0.5, 0.25])
    padded = np.pad(row, 1, mode="edge")
    out = np.convolve(padded, kernel, mode="valid")
    return float(out.mean())


futures = [blur_row.remote(y) for y in range(height)]
means = ray.get(futures)
result = {"width": width, "height": height, "meanIntensity": float(sum(means) / len(means))}
