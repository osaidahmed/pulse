def flat_calls
  a = 1
  b = 2
  c = a + b
  c
end

def pick_branch(x)
  if x > 10
    3
  elsif x > 5
    2
  else
    1
  end
end

def nested_guard(a, b, c)
  if a > 0
    if b > 0
      if c > 0
        return 1
      end
    end
  end
  0
end

def loop_filter(n)
  total = 0
  for i in 0..n
    if i % 2 == 0
      total += i
    end
  end
  total
end

def wide_params(a, b, c, d, e, f)
  a + b + c + d + e + f
end

def bool_blend(a, b, c, d)
  if (a > 0 && b > 0) || (c > 0 && d > 0)
    return 1
  end
  0
end
