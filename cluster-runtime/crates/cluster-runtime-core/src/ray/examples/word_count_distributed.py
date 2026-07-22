# Word Count (distributed via Ray)
# Packages: (stdlib only)
lines = ARGS[0]
from collections import Counter
import ray


@ray.remote
def count_line(line):
    return Counter(w.lower() for w in line.split())


futures = [count_line.remote(line) for line in lines]
partials = ray.get(futures)
total = Counter()
for p in partials:
    total.update(p)
top = total.most_common(10)
result = {"uniqueWords": len(total), "top": top, "totalWords": sum(total.values())}
