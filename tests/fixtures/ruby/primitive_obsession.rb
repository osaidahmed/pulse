def draw_rect(x, y, width, height)
  puts "Drawing at #{x},#{y} size #{width}x#{height}"
end

def configure_network(host, port, timeout, retries, buffer)
  { host: host, port: port, timeout: timeout, retries: retries, buffer: buffer }
end
