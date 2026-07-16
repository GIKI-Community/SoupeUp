from distributed import Client
import numpy as np
from PIL import Image
import os
import socket

# Connect to the scheduler
client = Client("tcp://192.168.18.55:8786")

WIDTH = 4096
HEIGHT = 4096
MAX_ITER = 1000

# Divide image into chunks
CHUNK_HEIGHT = 32


def render_chunk(y_start, y_end):
    import numpy as np
    import socket
    import os

    xmin, xmax = -2.5, 1.0
    ymin, ymax = -1.5, 1.5

    img = np.zeros((y_end - y_start, WIDTH), dtype=np.uint16)

    for py in range(y_start, y_end):
        cy = ymin + (py / HEIGHT) * (ymax - ymin)

        for px in range(WIDTH):
            cx = xmin + (px / WIDTH) * (xmax - xmin)

            z = 0j
            c = complex(cx, cy)

            i = 0
            while abs(z) <= 2 and i < MAX_ITER:
                z = z * z + c
                i += 1

            img[py - y_start, px] = i

    print(
        f"Finished rows {y_start}-{y_end} on "
        f"{socket.gethostname()} pid={os.getpid()}"
    )

    return y_start, img


# Submit one task per chunk
futures = []

for y in range(0, HEIGHT, CHUNK_HEIGHT):
    futures.append(
        client.submit(
            render_chunk,
            y,
            min(y + CHUNK_HEIGHT, HEIGHT),
            pure=False,
        )
    )

results = client.gather(futures)

# Assemble final image
final = np.zeros((HEIGHT, WIDTH), dtype=np.uint16)

for y, chunk in results:
    final[y : y + chunk.shape[0], :] = chunk

# Simple grayscale mapping
final = (255 * final / MAX_ITER).astype(np.uint8)

Image.fromarray(final).save("mandelbrot.png")

print("Saved mandelbrot.png")