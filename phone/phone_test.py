from distributed import Client
import time

client = Client("tcp://127.0.0.1:8786")

def work(x):
    import socket
    import os
    time.sleep(2)
    return {
        "value": x,
        "host": socket.gethostname(),
        "pid": os.getpid()
    }

futures = client.map(work, range(20))

for result in client.gather(futures):
    print(result)