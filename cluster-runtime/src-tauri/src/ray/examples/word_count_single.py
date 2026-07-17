# Word Count (single-node baseline)
# Packages: (stdlib only)
def user_fn(lines):
    from collections import Counter
    total = Counter()
    for line in lines:
        total.update(w.lower() for w in line.split())
    return {"uniqueWords": len(total), "top": total.most_common(10)}
