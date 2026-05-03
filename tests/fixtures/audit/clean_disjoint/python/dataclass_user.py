from dataclasses import dataclass, field


@dataclass(frozen=True)
class Point:
    x: float
    y: float
    label: str = field(default="")
