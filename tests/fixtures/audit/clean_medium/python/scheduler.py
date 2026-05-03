import heapq


def next_event(queue):
    return heapq.heappop(queue)
