module ProductionService where

data Request = Request
  { reqMethod :: String
  , reqPath :: String
  , reqBody :: String
  }

data Response = Response
  { resStatus :: Int
  , resBody :: String
  }

handleRequest :: Request -> Response
handleRequest req = case reqMethod req of
  "GET"    -> handleGet (reqPath req)
  "POST"   -> handlePost (reqPath req) (reqBody req)
  "DELETE" -> handleDelete (reqPath req)
  _        -> Response 405 "Method Not Allowed"

handleGet :: String -> Response
handleGet path = Response 200 ("GET " ++ path)

handlePost :: String -> String -> Response
handlePost path body = Response 201 ("POST " ++ path ++ " " ++ body)

handleDelete :: String -> Response
handleDelete path = Response 204 ("DELETE " ++ path)
