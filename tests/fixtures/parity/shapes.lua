local function flat_calls()
  local a = 1
  local b = 2
  local c = a + b
  return c
end

local function pick_branch(x)
  if x > 10 then
    return 3
  elseif x > 5 then
    return 2
  else
    return 1
  end
end

local function nested_guard(a, b, c)
  if a > 0 then
    if b > 0 then
      if c > 0 then
        return 1
      end
    end
  end
  return 0
end

local function loop_filter(n)
  local total = 0
  for i = 1, n do
    if i % 2 == 0 then
      total = total + i
    end
  end
  return total
end

local function wide_params(a, b, c, d, e, f)
  return a + b + c + d + e + f
end

local function bool_blend(a, b, c, d)
  if (a > 0 and b > 0) or (c > 0 and d > 0) then
    return 1
  end
  return 0
end

return {
  flat_calls = flat_calls,
  pick_branch = pick_branch,
  nested_guard = nested_guard,
  loop_filter = loop_filter,
  wide_params = wide_params,
  bool_blend = bool_blend,
}
