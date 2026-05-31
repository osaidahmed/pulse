-- A realistic Lua API service with constructor over-injection and complex routing.

local config = {
    host = "localhost",
    port = 8080,
    database_url = "postgres://localhost/app",
    redis_url = "redis://localhost",
    jwt_secret = "secret",
    log_level = "info",
    max_connections = 100,
    timeout_secs = 30,
    retry_count = 3,
    cache_ttl = 300,
    rate_limit = 1000,
}

local ApiService = {}

function ApiService:new(db, cache, auth, logger, metrics, validator, p7, p8, p9)
    self.db = db
    self.cache = cache
    self.auth = auth
    self.logger = logger
    self.metrics = metrics
    self.validator = validator
end

function ApiService:handleRequest(
    method, path, body, headers,
    query_params, auth_token, client_ip,
    request_id, trace_id, priority
)
    local response = { code = 200, headers = {} }

    if not auth_token then
        response.code = 401
        return response
    end

    local user = self.auth:validate(auth_token)
    if not user then
        response.code = 403
        return response
    end

    if method == "GET" then
        if user.role == "admin" then
            response.data = self.db:queryAll(path)
        elseif user.role == "user" then
            if user.verified then
                response.data = self.db:queryOwned(path, user.id)
            else
                response.code = 403
                response.error = "unverified"
            end
        else
            response.code = 403
        end
    elseif method == "POST" then
        if body and headers["content-type"] then
            if user.role == "admin" then
                if self.validator:validate(body) then
                    response.data = self.db:insert(path, body)
                else
                    response.code = 400
                    response.error = "validation_failed"
                end
            elseif user.role == "user" and user.verified then
                if self.validator:validate(body) then
                    response.data = self.db:insertOwned(path, body, user.id)
                else
                    response.code = 400
                end
            else
                response.code = 403
            end
        else
            response.code = 400
            response.error = "missing_body"
        end
    elseif method == "DELETE" then
        if user.role == "admin" then
            response.data = self.db:delete(path)
        else
            response.code = 403
        end
    end

    self.logger:info(request_id, method, path, response.code)
    return response
end

function ApiService:getUser(user_id)
    return self.db:query("users", user_id)
end

function ApiService:searchUsers(query)
    return self.db:search("users", query)
end

function ApiService:deleteUser(user_id)
    return self.db:delete("users/" .. user_id)
end
