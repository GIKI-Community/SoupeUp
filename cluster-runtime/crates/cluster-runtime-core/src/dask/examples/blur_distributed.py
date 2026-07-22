# Image Blur (distributed)
# Packages: numpy
width, height = ARGS
import numpy as np


def blur_row(y, w=width):
    row = np.sin(np.linspace(0, 8, w) + y * 0.05)
    kernel = np.array([0.25, 0.5, 0.25])
    padded = np.pad(row, 1, mode="edge")
    out = np.convolve(padded, kernel, mode="valid")
    return float(out.mean())


futures = client.map(blur_row, range(height))
means = client.gather(futures)
result = {"width": width, "height": height, "meanIntensity": float(sum(means) / len(means))}
