# Image Blur (single-node baseline)
# Packages: numpy
def user_fn(width, height):
    import numpy as np
    img = np.sin(np.linspace(0, 8, width * height).reshape(height, width))
    return {"meanIntensity": float(img.mean()), "width": width, "height": height}
