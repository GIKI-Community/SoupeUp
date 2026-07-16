import ray
import math

ray.init("ray://10.1.135.61:10001")

@ray.remote
def compute(n):
    total = 0
    for i in range(10_000_000):
        total += math.sqrt(i + n)
    return total

tasks = [compute.remote(i) for i in range(16)]
print(sum(ray.get(tasks)))