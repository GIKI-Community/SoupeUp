# Word Count (distributed)
# Packages: (stdlib only)
lines = ARGS[0]
from collections import Counter


def count_line(line):
    return Counter(w.lower() for w in line.split())


futures = client.map(count_line, lines)
partials = client.gather(futures)
total = Counter()
for p in partials:
    total.update(p)
top = total.most_common(10)
result = {"uniqueWords": len(total), "top": top, "totalWords": sum(total.values())}
