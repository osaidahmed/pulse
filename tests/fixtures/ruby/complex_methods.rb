def process_order(status, priority, amount, discount, tax)
  result = 0
  if status == 1
    if priority > 5
      result = amount * 2
    elsif priority > 3
      result = amount
    else
      result = amount / 2
    end
  elsif status == 2
    if discount > 0
      result = amount - discount
    end
  elsif status == 3
    case priority
    when 1
      result = amount + tax
    when 2
      result = amount - tax
    when 3
      result = amount
    end
  end
  result = 0 if result < 0
  result
end

def simple_helper(x)
  x + 1
end
