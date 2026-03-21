def bumpy_process(items)
  result = 0
  if items.length > 0
    if items[0] > 0
      if items[0] > 10
        result += 1
      end
    end
  end
  result += 1
  if items.length > 5
    if items[5] > 0
      if items[5] > 10
        result += 2
      end
    end
  end
  result += 2
  if items.length > 10
    if items[10] > 0
      if items[10] > 10
        result += 3
      end
    end
  end
  result
end
