def deeply_nested(items, threshold)
  count = 0
  items.each do |item|
    if item > threshold
      if item < 200
        if item != 128
          if count < 1000
            count += 1
          end
        end
      end
    end
  end
  count
end

def moderately_nested(x, y)
  if x > 0
    if y > 0
      return x + y
    end
  end
  0
end
