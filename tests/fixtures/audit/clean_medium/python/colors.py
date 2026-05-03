PALETTE = [(255, 0, 0), (0, 255, 0), (0, 0, 255)]


def hex_for(idx):
    r, g, b = PALETTE[idx % len(PALETTE)]
    return f"#{r:02x}{g:02x}{b:02x}"
