import ray


print("hi")


@ray.remote
def add():
    num = 1 + 1
    return num

# test = ray.remote(add)

if __name__ == "__main__":
    ray.init(address="192.168.30.44:6379")
    future = add.remote()
    print(ray.get(future))