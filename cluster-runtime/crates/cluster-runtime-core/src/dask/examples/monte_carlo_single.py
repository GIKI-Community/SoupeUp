# Monte Carlo π Estimation (single-node baseline)
# Packages: (stdlib only)
def user_fn(samples):
    import random
    inside = 0
    for _ in range(samples):
        x = random.random()
        y = random.random()
        if x * x + y * y <= 1.0:
            inside += 1
    return {"pi": 4.0 * inside / samples, "samples": samples}
