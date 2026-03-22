function createRect(x, y, width, height, color, opacity)
    local rect = {}
    rect.x = x
    rect.y = y
    rect.width = width
    rect.height = height
    rect.color = color
    rect.opacity = opacity
    return rect
end

function simpleAdd(a, b)
    return a + b
end
