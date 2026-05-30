class AppConfig
  attr_accessor :host, :port, :database_url, :redis_url, :jwt_secret,
                :log_level, :max_connections, :timeout_secs, :retry_count,
                :cache_ttl, :rate_limit, :upload_dir, :cors_origin
end

def dispatch_command(cmd)
  case cmd
  when "start" then "starting"
  when "stop" then "stopping"
  when "restart" then "restarting"
  when "status" then "checking"
  when "deploy" then "deploying"
  when "rollback" then "rolling back"
  end
end

def process_event(data)
  a = data[0]
  b = data[1]
  c = data[2]
  d = data[3]
  e = data[4]
  f = data[5]
  g = data[6]
  h = data[7]
  return -1 if a > 100
  return -2 if b > 200
  result = a + b + c + d
  extra = e + f + g + h
  result + extra
end
