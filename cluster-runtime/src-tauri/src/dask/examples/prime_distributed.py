# Prime Number Search (distributed)
# Packages: (stdlib only)
limit = ARGS[0]


def is_prime(n):
    if n < 2:
        return False
    if n % 2 == 0:
        return n == 2
    i = 3
    while i * i <= n:
        if n % i == 0:
            return False
        i += 2
    return True


futures = client.map(is_prime, range(limit))
flags = client.gather(futures)
count = sum(1 for f in flags if f)
result = {"limit": limit, "primeCount": count}
