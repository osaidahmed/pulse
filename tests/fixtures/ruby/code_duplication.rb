def process_alpha(data)
  result = 0
  data.each do |item|
    if item > 100
      result += 2
    else
      result += 1
    end
  end
  result
end

def process_beta(data)
  result = 0
  data.each do |item|
    if item > 100
      result += 2
    else
      result += 1
    end
  end
  result
end

def process_gamma(items)
  count = 0
  items.each do |val|
    if val > 100
      count += 2
    else
      count += 1
    end
  end
  count
end
