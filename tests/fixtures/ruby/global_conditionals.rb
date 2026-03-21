$global_state = 0

def init
  $global_state = 1
end

if $global_state > 0
  if $global_state < 100
    $global_state += 1
  end
end
