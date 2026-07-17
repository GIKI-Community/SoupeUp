# Prime Number Search (single-node baseline)
# Packages: (stdlib only)
def user_fn(limit):
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
    count = sum(1 for n in range(limit) if is_prime(n))
    return {"limit": limit, "primeCount": count}
