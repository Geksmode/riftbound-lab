"""Agents. RandomAgent is used to fuzz the engine (every legal action must be executable)."""
import random


class RandomAgent:
    def __init__(s, seed=None):
        s.rng = random.Random(seed)

    def start(s, g):
        pass

    def mulligan(s, g, pid):
        h = g.p[pid].hand
        return s.rng.sample(h, s.rng.randint(0, 2))

    def decide(s, g, d):
        opts = d.options
        if d.kind == "main" and len(opts) > 1 and s.rng.random() < 0.85:
            opts = [o for o in opts if o[0] != "end"]
        if d.kind != "main" and s.rng.random() < 0.6:
            return ("pass",)
        return s.rng.choice(opts)

    def choose(s, g, pid, kind, options, ctx):
        if kind == "damage_order":
            t = list(options[0]); s.rng.shuffle(t); return t
        return s.rng.choice(options)
